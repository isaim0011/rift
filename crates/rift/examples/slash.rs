//! Slash commands, end to end, with typed command definitions.
//!
//!   /ping           -> "pong" immediately (public)
//!   /slow           -> defers (private "thinking..."), waits 2s, then edits the reply
//!   /echo text:..   -> repeats your text; optional shout:true makes it CAPS
//!
//! PowerShell:  $env:DISCORD_TOKEN="your-token"; cargo run -p rift --example slash
//!
//! The app must be installed with the `applications.commands` scope (the `bot`
//! scope includes it). Interactions need no privileged intent.

use rift::{
    AllowedMentions, Client, CommandSpec, Intents, Interaction, InteractionMessage, InteractionResponse,
    InteractionType, OptionSpec, Snowflake,
};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct Id {
    id: String,
}

#[derive(Deserialize)]
struct Ready {
    user: Id,
    application: Option<Id>,
}

#[tokio::main]
async fn main() {
    let token = std::env::var("DISCORD_TOKEN").expect("set DISCORD_TOKEN");
    let mut client = Client::builder(token, Intents::GUILDS).build().await.expect("failed to start");
    let mut registered = false;

    loop {
        tokio::select! {
            event = client.next_event() => {
                let Some(e) = event else { break };
                match e.name.as_str() {
                    "READY" if !registered => {
                        let Ok(ready) = serde_json::from_str::<Ready>(e.data.get()) else { continue };
                        let app_id = ready.application.map(|a| a.id).unwrap_or(ready.user.id);
                        let Ok(app_id) = app_id.parse::<Snowflake>() else { continue };
                        registered = true;

                        let rest = client.rest().clone();
                        tokio::spawn(async move {
                            // Validated locally, then sent as an idempotent bulk overwrite.
                            let commands = [
                                CommandSpec::slash("ping", "Replies with pong"),
                                CommandSpec::slash("slow", "Replies after a deferred 2 second delay"),
                                CommandSpec::slash("echo", "Repeats your text")
                                    .option(OptionSpec::string("text", "What to repeat").required().max_length(200))
                                    .option(OptionSpec::boolean("shout", "Repeat it in capitals")),
                            ];
                            match rest.bulk_overwrite_global_commands(app_id, &commands).await {
                                Ok(done) => println!("registered {} commands", done.len()),
                                Err(err) => eprintln!("command registration failed: {err}"),
                            }
                        });
                    }
                    "INTERACTION_CREATE" => {
                        let Ok(i) = serde_json::from_str::<Interaction>(e.data.get()) else { continue };
                        if i.kind != InteractionType::ApplicationCommand { continue }
                        let Ok(cmd) = i.command() else { continue };

                        // Respond from a task so the event loop never waits on the network.
                        let rest = client.rest().clone();
                        tokio::spawn(async move {
                            let result = match cmd.name.as_str() {
                                "ping" => {
                                    let r = InteractionResponse::message(InteractionMessage::new().content("pong"));
                                    rest.create_interaction_response(i.id, &i.token, &r).await
                                }
                                "echo" => {
                                    let text = cmd.option("text").and_then(|o| o.as_str()).unwrap_or("");
                                    let shout = cmd.option("shout").and_then(|o| o.as_bool()).unwrap_or(false);
                                    let out = if shout { text.to_uppercase() } else { text.to_owned() };
                                    // User text can contain mentions: never let it ping anyone.
                                    let msg = InteractionMessage::new()
                                        .content(out)
                                        .allowed_mentions(AllowedMentions::none());
                                    rest.create_interaction_response(i.id, &i.token, &InteractionResponse::message(msg)).await
                                }
                                "slow" => {
                                    let first = rest
                                        .create_interaction_response(i.id, &i.token, &InteractionResponse::defer(true))
                                        .await;
                                    if first.is_ok() {
                                        tokio::time::sleep(Duration::from_secs(2)).await;
                                        rest.edit_original_response(
                                            i.application_id,
                                            &i.token,
                                            &InteractionMessage::new().content("done after 2 seconds"),
                                        )
                                        .await
                                        .map(|_| ())
                                    } else {
                                        first
                                    }
                                }
                                _ => Ok(()),
                            };
                            if let Err(err) = result {
                                eprintln!("/{} failed: {err}", cmd.name);
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
