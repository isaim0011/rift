//! Fetch /gateway/bot and print what it says.
//!
//! PowerShell:  $env:DISCORD_TOKEN="your-token"; cargo run -p rift-rest --example gateway_bot
//! This does not open a gateway session, so it spends no Identify.

use rift_rest::RestClient;

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");
    let client = RestClient::new(&token, "https://github.com/your-name/rift", env!("CARGO_PKG_VERSION"))
        .expect("could not build HTTP client");

    match client.gateway_bot().await {
        Ok(g) => {
            println!("gateway url:        {}", g.url);
            println!("recommended shards: {}", g.shards);
            println!("session limit:      {:?}", g.session_start_limit);
            println!("start waves:        {:?}", g.start_waves());
        }
        Err(e) => eprintln!("error: {e}"),
    }
}
