//! Connect one shard and print every event name.
//!
//! PowerShell:  $env:DISCORD_TOKEN="your-token"; cargo run -p rift-runtime --example ping
//! bash:        DISCORD_TOKEN=your-token cargo run -p rift-runtime --example ping

use rift_core::{Capabilities, Intents};
use rift_gateway::IdentifyBudget;
use rift_runtime::{now_ms, run_shard, Command, ShardConfig};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");
    let mut config = ShardConfig::new(token, Intents::GUILDS | Intents::GUILD_MESSAGES);

    // Optional: test Discord's channel-obfuscation opt-in (temporary, testing-only).
    // PowerShell: $env:RIFT_CHANNEL_OBFUSCATION="1"
    if std::env::var("RIFT_CHANNEL_OBFUSCATION").is_ok() {
        config = config.with_capabilities(Capabilities::CHANNEL_OBFUSCATION);
        println!("capability CHANNEL_OBFUSCATION enabled");
    }

    // Until rift-http fetches /gateway/bot, assume the standard 1000/day.
    let budget = Arc::new(Mutex::new(IdentifyBudget::from_session_start_limit(
        1000, 1000, 86_400_000, now_ms(), 10,
    )));

    let (events_tx, mut events_rx) = mpsc::unbounded_channel();
    let (cmd_tx, cmd_rx) = mpsc::channel(64);
    let shard = tokio::spawn(run_shard(config, budget, events_tx, cmd_rx));

    loop {
        tokio::select! {
            ev = events_rx.recv() => match ev {
                Some(e) => println!("#{:<5} {}", e.seq, e.name),
                None => break,
            },
            _ = tokio::signal::ctrl_c() => {
                println!("shutting down...");
                let _ = cmd_tx.send(Command::Shutdown).await;
                break;
            }
        }
    }
    println!("shard finished: {:?}", shard.await);
}
