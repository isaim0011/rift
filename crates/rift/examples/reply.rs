//! Reply "pong" whenever someone @mentions the bot.
//!
//! Needs no privileged intent: Discord always sends message content for
//! messages that mention the app.
//!
//! PowerShell:  $env:DISCORD_TOKEN="your-token"; cargo run -p rift --example reply
//! Invite the bot to a server, then write "@YourBot ping" in a channel it can see.

use rift::{AllowedMentions, Client, CreateMessage, Intents, Snowflake};
use serde::Deserialize;

#[derive(Deserialize)]
struct User {
    id: String,
    #[serde(default)]
    bot: bool,
}

#[derive(Deserialize)]
struct Ready {
    user: User,
}

#[derive(Deserialize)]
struct MessageCreate {
    id: String,
    channel_id: String,
    #[serde(default)]
    mentions: Vec<User>,
    author: Option<User>,
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");
    let mut client = Client::builder(token, Intents::GUILDS | Intents::GUILD_MESSAGES)
        .build()
        .await
        .expect("failed to start");

    let mut me: Option<String> = None;

    loop {
        tokio::select! {
            event = client.next_event() => {
                let Some(e) = event else { break };
                match e.name.as_str() {
                    "READY" => {
                        if let Ok(r) = serde_json::from_str::<Ready>(e.data.get()) {
                            println!("ready as user {}", r.user.id);
                            me = Some(r.user.id);
                        }
                    }
                    "MESSAGE_CREATE" => {
                        let Ok(m) = serde_json::from_str::<MessageCreate>(e.data.get()) else { continue };
                        if m.author.as_ref().is_some_and(|a| a.bot) { continue; }
                        if !m.mentions.iter().any(|u| Some(&u.id) == me.as_ref()) { continue; }
                        let (Ok(channel), Ok(message)) =
                            (m.channel_id.parse::<Snowflake>(), m.id.parse::<Snowflake>()) else { continue };

                        let rest = client.rest().clone();
                        tokio::spawn(async move {
                            let reply = CreateMessage::new()
                                .content("pong")
                                .reply_to(message)
                                .allowed_mentions(AllowedMentions::none());
                            match rest.create_message(channel, &reply).await {
                                Ok(sent) => println!("replied (message {})", sent.id),
                                Err(err) => eprintln!("could not reply: {err}"),
                            }
                        });
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                println!("shutting down...");
                break;
            }
        }
    }
    let _ = client.shutdown().await;
}
