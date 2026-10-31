//! Run the whole stack: REST + all shards, printing every event.
//!
//! PowerShell:  $env:DISCORD_TOKEN="your-token"; cargo run -p rift --example bot
//! Spends one Identify per shard. Ctrl+C shuts down cleanly.

use rift::{Client, Intents};

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");

    let mut client = Client::builder(token, Intents::GUILDS | Intents::GUILD_MESSAGES)
        .contact_url("https://github.com/your-name/rift")
        .build()
        .await
        .expect("failed to start");

    println!("starting {} shard(s)...", client.num_shards());

    loop {
        tokio::select! {
            event = client.next_event() => match event {
                Some(e) => println!("[shard {}] #{:<5} {}", e.shard_id, e.seq, e.name),
                None => break,
            },
            _ = tokio::signal::ctrl_c() => {
                println!("shutting down...");
                break;
            }
        }
    }

    if let Err(e) = client.shutdown().await {
        eprintln!("stopped with error: {e}");
    }
}
