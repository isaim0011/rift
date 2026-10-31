//! Launches shards in waves and enforces the failure policy.
//!
//! Policy: any shard stopping with an error stops the whole cluster. A partial
//! cluster silently misses the guilds that hash to the dead shard.

use rift_runtime::Error;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinSet;
use tokio::time::{sleep_until, Instant};

pub(crate) type ShardResult = (u32, Result<(), Error>);

/// Run until every shard has finished (or been stopped).
///
/// * `waves` come from `rift_core::shard::start_waves`; each wave starts `gap`
///   after the previous one (Discord limits Identifies per 5 seconds).
/// * `start` spawns one shard into the set.
/// * `stop_all` asks every shard to shut down cleanly.
/// * Anything arriving on `stop_rx` (or the channel closing) is a stop request.
pub(crate) async fn supervise(
    waves: Vec<Vec<u32>>,
    gap: Duration,
    mut start: impl FnMut(u32, &mut JoinSet<ShardResult>),
    stop_all: impl Fn(),
    mut stop_rx: mpsc::UnboundedReceiver<()>,
) -> Result<(), Error> {
    let mut set: JoinSet<ShardResult> = JoinSet::new();
    let mut next_wave = 0usize;
    let mut next_at = Instant::now();
    let mut stopping = false;
    let mut first_err: Option<Error> = None;

    loop {
        let launching = !stopping && next_wave < waves.len();
        if !launching && set.is_empty() {
            break;
        }

        tokio::select! {
            _ = sleep_until(next_at), if launching => {
                tracing::info!(
                    wave = next_wave + 1,
                    total_waves = waves.len(),
                    shard_count = waves[next_wave].len(),
                    shards = ?waves[next_wave],
                    "launching shard wave"
                );
                for &id in &waves[next_wave] {
                    start(id, &mut set);
                }
                next_wave += 1;
                next_at = Instant::now() + gap;
            }
            joined = set.join_next(), if !set.is_empty() => {
                let failure = match joined {
                    Some(Ok((_, Ok(())))) | None => None,
                    Some(Ok((id, Err(e)))) => {
                        tracing::warn!(shard_id = id, error = %e, "shard stopped with error");
                        Some(e)
                    }
                    Some(Err(join_err)) => {
                        tracing::error!(error = %join_err, "shard task failed or panicked");
                        Some(Error::Transport(format!("shard task failed: {join_err}")))
                    }
                };
                if let Some(e) = failure {
                    if first_err.is_none() {
                        first_err = Some(e);
                    }
                    if !stopping {
                        stopping = true;
                        tracing::warn!("stopping all shards due to shard failure");
                        stop_all();
                    }
                }
            }
            _ = stop_rx.recv(), if !stopping => {
                stopping = true;
                tracing::info!("received stop request; shutting down all shards");
                stop_all();
            }
        }
    }

    first_err.map_or(Ok(()), Err)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rift_core::CloseCode;
    use std::sync::{Arc, Mutex};
    use tokio::sync::watch;
    use tokio::time::sleep;

    #[derive(Clone, Copy)]
    enum Plan {
        UntilStopped,
        OkAfter(Duration),
        FailAfter(Duration),
        Panic,
    }

    type Started = Arc<Mutex<Vec<(Duration, u32)>>>;

    /// Build `start`/`stop_all` closures that spawn fake shards following `plans`.
    fn harness(
        plans: Vec<Plan>,
    ) -> (
        Started,
        impl FnMut(u32, &mut JoinSet<ShardResult>),
        impl Fn(),
    ) {
        let t0 = Instant::now();
        let started: Started = Arc::new(Mutex::new(Vec::new()));
        let (stop_tx, stop_rx) = watch::channel(false);

        let log = started.clone();
        let start = move |id: u32, set: &mut JoinSet<ShardResult>| {
            log.lock().unwrap().push((t0.elapsed(), id));
            let mut rx = stop_rx.clone();
            let plan = plans[id as usize];
            set.spawn(async move {
                match plan {
                    Plan::UntilStopped => {
                        let _ = rx.wait_for(|stopped| *stopped).await;
                        (id, Ok(()))
                    }
                    Plan::OkAfter(d) => {
                        sleep(d).await;
                        (id, Ok(()))
                    }
                    Plan::FailAfter(d) => {
                        sleep(d).await;
                        (id, Err(Error::Fatal(CloseCode::AuthenticationFailed)))
                    }
                    Plan::Panic => panic!("fake shard panic"),
                }
            });
        };
        let stop_all = move || {
            let _ = stop_tx.send(true);
        };
        (started, start, stop_all)
    }

    fn ids(started: &Started) -> Vec<u32> {
        started.lock().unwrap().iter().map(|&(_, id)| id).collect()
    }

    #[tokio::test(start_paused = true)]
    async fn waves_start_in_order_one_gap_apart() {
        let ok = Plan::OkAfter(Duration::from_secs(1));
        let (started, start, stop_all) = harness(vec![ok, ok, ok]);
        let (_keep, stop_rx) = mpsc::unbounded_channel();

        let r = supervise(vec![vec![0, 1], vec![2]], Duration::from_secs(5), start, stop_all, stop_rx).await;
        assert!(r.is_ok());

        let s = started.lock().unwrap();
        assert_eq!(s.iter().map(|&(_, id)| id).collect::<Vec<_>>(), vec![0, 1, 2]);
        // Wave 0 starts immediately, together.
        assert!(s[0].0 < Duration::from_millis(10) && s[1].0 < Duration::from_millis(10));
        // Wave 1 starts one gap later.
        assert!(s[2].0 >= Duration::from_secs(5) && s[2].0 < Duration::from_millis(5_050));
    }

    #[tokio::test(start_paused = true)]
    async fn one_fatal_shard_stops_the_cluster_and_skips_later_waves() {
        let plans = vec![
            Plan::UntilStopped,
            Plan::FailAfter(Duration::from_secs(1)),
            Plan::UntilStopped,
        ];
        let (started, start, stop_all) = harness(plans);
        let (_keep, stop_rx) = mpsc::unbounded_channel();

        let r = supervise(
            vec![vec![0], vec![1], vec![2]],
            Duration::from_secs(5),
            start,
            stop_all,
            stop_rx,
        )
        .await;

        assert!(matches!(r, Err(Error::Fatal(CloseCode::AuthenticationFailed))));
        // Shard 2 would have started at t=10s, but the cluster was already stopping.
        assert_eq!(ids(&started), vec![0, 1]);
    }

    #[tokio::test(start_paused = true)]
    async fn user_stop_request_shuts_down_cleanly() {
        let (started, start, stop_all) = harness(vec![Plan::UntilStopped, Plan::UntilStopped]);
        let (stop_tx, stop_rx) = mpsc::unbounded_channel();
        tokio::spawn(async move {
            sleep(Duration::from_secs(1)).await;
            let _ = stop_tx.send(());
        });

        let r = supervise(vec![vec![0], vec![1]], Duration::from_secs(5), start, stop_all, stop_rx).await;
        assert!(r.is_ok());
        assert_eq!(ids(&started), vec![0]);
    }

    #[tokio::test(start_paused = true)]
    async fn dropping_the_stop_channel_counts_as_a_stop() {
        let (started, start, stop_all) = harness(vec![Plan::UntilStopped]);
        let (stop_tx, stop_rx) = mpsc::unbounded_channel::<()>();
        drop(stop_tx);

        let r = supervise(vec![vec![0]], Duration::from_secs(5), start, stop_all, stop_rx).await;
        assert!(r.is_ok());
        // Stopping was requested at once; the cluster never leaves the ground.
        assert!(ids(&started).len() <= 1);
    }

    #[tokio::test(start_paused = true)]
    async fn a_panicking_shard_is_reported_as_an_error() {
        let (_started, start, stop_all) = harness(vec![Plan::Panic]);
        let (_keep, stop_rx) = mpsc::unbounded_channel();
        let r = supervise(vec![vec![0]], Duration::from_secs(5), start, stop_all, stop_rx).await;
        assert!(matches!(r, Err(Error::Transport(_))));
    }
}
