//! End-to-end benchmark harness connecting a real [`rift_runtime::run_shard`]
//! to the local [`crate::server::MockGateway`] over WebSocket + `zstd-stream`.

use crate::server::{MockGateway, MockGatewayConfig};
use rift_cache::{Cache, CacheConfig};
use rift_gateway::IdentifyBudget;
use rift_model::GatewayEvent;
use rift_runtime::{event_channel, run_shard, Command, EventBufferPolicy, ShardConfig};
use serde::{Deserialize, Serialize};
use std::io;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Configuration for an end-to-end benchmark run.
#[derive(Clone, Debug)]
pub struct HarnessConfig {
    /// Number of dispatch events to stream after `READY` (default 5,000).
    pub event_count: usize,
    /// Number of members in the initial `GUILD_CREATE` event (default 100).
    pub guild_members: usize,
    /// Cache configuration for in-memory state updates during ingestion.
    pub cache_config: CacheConfig,
    /// Zstd compression level (default 3).
    pub zstd_level: i32,
    /// Bot token to pass in ShardConfig.
    pub token: String,
}

impl Default for HarnessConfig {
    fn default() -> Self {
        Self {
            event_count: 5_000,
            guild_members: 100,
            cache_config: CacheConfig::all(),
            zstd_level: 3,
            token: "rift-bench-token".to_string(),
        }
    }
}

/// Metrics measured during an end-to-end benchmark execution.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct HarnessReport {
    /// Total number of dispatch events ingested (excluding READY).
    pub total_events: usize,
    /// Wall-clock duration from shard start to READY received.
    pub time_to_ready: Duration,
    /// Total wall-clock duration to ingest the N events.
    pub total_duration: Duration,
    /// End-to-end throughput in events per second.
    pub events_per_second: f64,
    /// Process CPU time spent per 1,000 events.
    pub cpu_time_per_1k: Duration,
    /// 50th percentile (median) per-event ingestion latency in nanoseconds.
    pub p50_latency_ns: u64,
    /// 99th percentile per-event ingestion latency in nanoseconds.
    pub p99_latency_ns: u64,
}

/// Summary statistics (Min, Median, Max) computed over multiple harness runs.
#[derive(Clone, Debug)]
pub struct MultiRunSummary {
    /// Number of runs aggregated.
    pub run_count: usize,
    /// Events per run.
    pub events_per_run: usize,
    /// Wall-clock time to READY: (min, median, max).
    pub time_to_ready: (Duration, Duration, Duration),
    /// Throughput in events/second: (min, median, max).
    pub events_per_second: (f64, f64, f64),
    /// CPU time per 1,000 events: (min, median, max).
    pub cpu_time_per_1k: (Duration, Duration, Duration),
    /// p50 ingestion latency in nanoseconds: (min, median, max).
    pub p50_latency_ns: (u64, u64, u64),
    /// p99 ingestion latency in nanoseconds: (min, median, max).
    pub p99_latency_ns: (u64, u64, u64),
}

impl MultiRunSummary {
    /// Generates a clean GitHub Flavored Markdown table representing the benchmark metrics.
    pub fn render_markdown_table(&self) -> String {
        let fmt_dur = |d: Duration| -> String {
            if d.as_micros() < 1000 {
                format!("{:.2} \u{03bc}s", d.as_nanos() as f64 / 1_000.0)
            } else if d.as_millis() < 1000 {
                format!("{:.2} ms", d.as_secs_f64() * 1_000.0)
            } else {
                format!("{:.3} s", d.as_secs_f64())
            }
        };

        let fmt_ns = |ns: u64| -> String {
            if ns < 1_000 {
                format!("{ns} ns")
            } else if ns < 1_000_000 {
                format!("{:.2} \u{03bc}s", ns as f64 / 1_000.0)
            } else {
                format!("{:.2} ms", ns as f64 / 1_000_000.0)
            }
        };

        let fmt_eps = |v: f64| -> String {
            format!("{:.0}", v)
        };

        let mut out = String::new();
        out.push_str("| Metric | Min | Median | Max |\n");
        out.push_str("|---|---|---|---|\n");
        out.push_str(&format!(
            "| **Time to READY** | {} | {} | {} |\n",
            fmt_dur(self.time_to_ready.0),
            fmt_dur(self.time_to_ready.1),
            fmt_dur(self.time_to_ready.2)
        ));
        out.push_str(&format!(
            "| **Events / sec** | {} ev/s | {} ev/s | {} ev/s |\n",
            fmt_eps(self.events_per_second.0),
            fmt_eps(self.events_per_second.1),
            fmt_eps(self.events_per_second.2)
        ));
        out.push_str(&format!(
            "| **CPU / 1,000 events** | {} | {} | {} |\n",
            fmt_dur(self.cpu_time_per_1k.0),
            fmt_dur(self.cpu_time_per_1k.1),
            fmt_dur(self.cpu_time_per_1k.2)
        ));
        out.push_str(&format!(
            "| **p50 Latency** | {} | {} | {} |\n",
            fmt_ns(self.p50_latency_ns.0),
            fmt_ns(self.p50_latency_ns.1),
            fmt_ns(self.p50_latency_ns.2)
        ));
        out.push_str(&format!(
            "| **p99 Latency** | {} | {} | {} |\n",
            fmt_ns(self.p99_latency_ns.0),
            fmt_ns(self.p99_latency_ns.1),
            fmt_ns(self.p99_latency_ns.2)
        ));

        out
    }
}

/// The end-to-end benchmark harness.
pub struct Harness;

impl Harness {
    /// Executes a single benchmark run with the given configuration.
    pub async fn run(config: HarnessConfig) -> io::Result<HarnessReport> {
        let gateway = MockGateway::start(MockGatewayConfig {
            heartbeat_interval_ms: 41_250,
            event_count: config.event_count,
            guild_members: config.guild_members,
            zstd_level: config.zstd_level,
            custom_events: None,
            uncompressed: false,
        })
        .await?;

        let intents = rift_core::Intents::from_bits(rift_core::Intents::ALL_KNOWN).unwrap_or_default();
        let mut shard_config = ShardConfig::new(config.token.clone(), intents);
        shard_config.gateway_url = gateway.url();
        shard_config.event_buffer_policy = EventBufferPolicy::Unbounded;

        let (events_tx, mut events_rx) = event_channel(shard_config.event_buffer_policy);
        let (cmd_tx, cmd_rx) = tokio::sync::mpsc::channel(64);
        let budget = Arc::new(Mutex::new(IdentifyBudget::from_session_start_limit(
            1000, 1000, 86_400_000, 0, 0,
        )));
        let mut cache = Cache::new(config.cache_config);

        let cpu_start = process_cpu_time();
        let shard_start = Instant::now();

        let shard_task = tokio::spawn(async move {
            run_shard(shard_config, budget, events_tx, cmd_rx).await
        });

        // 1. Wait for READY event
        let mut time_to_ready = Duration::ZERO;
        while let Some(event) = events_rx.recv().await {
            if event.name == "READY" {
                time_to_ready = shard_start.elapsed();
                if let Ok(typed) = GatewayEvent::parse(&event.name, &event.data) {
                    cache.update(&typed);
                }
                break;
            }
        }

        // 2. Measure end-to-end ingestion of N dispatch events
        let burst_start = Instant::now();
        let mut latencies = Vec::with_capacity(config.event_count);
        let mut events_ingested = 0;

        while events_ingested < config.event_count {
            let event = match events_rx.recv().await {
                Some(e) => e,
                None => break,
            };

            let t0 = Instant::now();
            if let Ok(typed) = GatewayEvent::parse(&event.name, &event.data) {
                cache.update(&typed);
            }
            let elapsed_ns = t0.elapsed().as_nanos() as u64;
            latencies.push(elapsed_ns);
            events_ingested += 1;
        }

        let burst_duration = burst_start.elapsed();
        let cpu_end = process_cpu_time();

        // 3. Clean shutdown
        let _ = cmd_tx.send(Command::Shutdown).await;
        let _ = shard_task.await;
        gateway.shutdown().await;

        // 4. Calculate metrics
        let events_per_second = if burst_duration.as_secs_f64() > 0.0 {
            events_ingested as f64 / burst_duration.as_secs_f64()
        } else {
            0.0
        };

        latencies.sort_unstable();
        let p50_latency_ns = if !latencies.is_empty() {
            latencies[latencies.len() / 2]
        } else {
            0
        };
        let p99_latency_ns = if !latencies.is_empty() {
            latencies[(latencies.len() * 99) / 100]
        } else {
            0
        };

        let cpu_delta = cpu_end.saturating_sub(cpu_start);
        let cpu_time_per_1k = if events_ingested > 0 {
            if cpu_delta > Duration::ZERO {
                Duration::from_secs_f64(cpu_delta.as_secs_f64() / (events_ingested as f64) * 1000.0)
            } else {
                Duration::from_secs_f64(burst_duration.as_secs_f64() / (events_ingested as f64) * 1000.0)
            }
        } else {
            Duration::ZERO
        };

        Ok(HarnessReport {
            total_events: events_ingested,
            time_to_ready,
            total_duration: burst_duration,
            events_per_second,
            cpu_time_per_1k,
            p50_latency_ns,
            p99_latency_ns,
        })
    }

    /// Aggregates multiple harness reports into min, median, and max metrics.
    pub fn aggregate(reports: &[HarnessReport]) -> Option<MultiRunSummary> {
        if reports.is_empty() {
            return None;
        }

        let n = reports.len();
        let events_per_run = reports[0].total_events;

        let mut ttr: Vec<Duration> = reports.iter().map(|r| r.time_to_ready).collect();
        ttr.sort();
        let time_to_ready = (ttr[0], ttr[n / 2], ttr[n - 1]);

        let mut eps: Vec<f64> = reports.iter().map(|r| r.events_per_second).collect();
        eps.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let events_per_second = (eps[0], eps[n / 2], eps[n - 1]);

        let mut cpu: Vec<Duration> = reports.iter().map(|r| r.cpu_time_per_1k).collect();
        cpu.sort();
        let cpu_time_per_1k = (cpu[0], cpu[n / 2], cpu[n - 1]);

        let mut p50: Vec<u64> = reports.iter().map(|r| r.p50_latency_ns).collect();
        p50.sort();
        let p50_latency_ns = (p50[0], p50[n / 2], p50[n - 1]);

        let mut p99: Vec<u64> = reports.iter().map(|r| r.p99_latency_ns).collect();
        p99.sort();
        let p99_latency_ns = (p99[0], p99[n / 2], p99[n - 1]);

        Some(MultiRunSummary {
            run_count: n,
            events_per_run,
            time_to_ready,
            events_per_second,
            cpu_time_per_1k,
            p50_latency_ns,
            p99_latency_ns,
        })
    }
}

/// Measures current process CPU execution time (user + system) cross-platform.
#[cfg(windows)]
pub fn process_cpu_time() -> Duration {
    #[repr(C)]
    struct FileTime {
        dw_low_date_time: u32,
        dw_high_date_time: u32,
    }
    extern "system" {
        fn GetCurrentProcess() -> *mut std::ffi::c_void;
        fn GetProcessTimes(
            h_process: *mut std::ffi::c_void,
            creation_time: *mut FileTime,
            exit_time: *mut FileTime,
            kernel_time: *mut FileTime,
            user_time: *mut FileTime,
        ) -> i32;
    }
    unsafe {
        let mut creation = FileTime { dw_low_date_time: 0, dw_high_date_time: 0 };
        let mut exit = FileTime { dw_low_date_time: 0, dw_high_date_time: 0 };
        let mut kernel = FileTime { dw_low_date_time: 0, dw_high_date_time: 0 };
        let mut user = FileTime { dw_low_date_time: 0, dw_high_date_time: 0 };
        if GetProcessTimes(GetCurrentProcess(), &mut creation, &mut exit, &mut kernel, &mut user) != 0 {
            let k = ((kernel.dw_high_date_time as u64) << 32) | (kernel.dw_low_date_time as u64);
            let u = ((user.dw_high_date_time as u64) << 32) | (user.dw_low_date_time as u64);
            Duration::from_nanos((k + u) * 100)
        } else {
            Duration::ZERO
        }
    }
}

/// Measures current process CPU execution time on Unix systems.
#[cfg(unix)]
pub fn process_cpu_time() -> Duration {
    let mut usage = std::mem::MaybeUninit::<libc::rusage>::uninit();
    let ret = unsafe { libc::getrusage(libc::RUSAGE_SELF, usage.as_mut_ptr()) };
    if ret == 0 {
        let usage = unsafe { usage.assume_init() };
        let user = Duration::new(usage.ru_utime.tv_sec as u64, (usage.ru_utime.tv_usec * 1000) as u32);
        let sys = Duration::new(usage.ru_stime.tv_sec as u64, (usage.ru_stime.tv_usec * 1000) as u32);
        user + sys
    } else {
        Duration::ZERO
    }
}

/// Fallback for unsupported operating systems.
#[cfg(not(any(windows, unix)))]
pub fn process_cpu_time() -> Duration {
    Duration::ZERO
}
