use rift_core::{Capabilities, Intents};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

/// Gateway event returned to JavaScript/Bun caller.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ClientEventOutput {
    /// Shard that received the event.
    pub shard_id: u32,
    /// Gateway dispatch event name (e.g. "MESSAGE_CREATE").
    pub name: String,
    /// Raw JSON string payload (`d` object).
    pub payload_json: String,
}

/// Outgoing command sent to gateway shard.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ShardCommand {
    pub shard_id: u32,
    pub opcode: u8,
    pub payload_json: String,
}

/// High-level async client wrapping Rift client event loop and dispatch routing.
pub struct NodeRiftClient {
    #[allow(dead_code)]
    token: String,
    intents: u32,
    capabilities: u32,
    event_tx: mpsc::UnboundedSender<ClientEventOutput>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<ClientEventOutput>>>,
    command_tx: mpsc::UnboundedSender<ShardCommand>,
    #[allow(dead_code)]
    command_rx: Arc<Mutex<mpsc::UnboundedReceiver<ShardCommand>>>,
    stopped: Arc<Mutex<bool>>,
}

impl NodeRiftClient {
    /// Creates a new client instance configured with bot token, intents, and capabilities.
    pub fn new(token: String, intents: Option<u32>, capabilities: Option<u32>) -> Self {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let (command_tx, command_rx) = mpsc::unbounded_channel();
        Self {
            token,
            intents: intents.unwrap_or(Intents::ALL_KNOWN),
            capabilities: capabilities.unwrap_or(Capabilities::NONE.bits()),
            event_tx,
            event_rx: Arc::new(Mutex::new(event_rx)),
            command_tx,
            command_rx: Arc::new(Mutex::new(command_rx)),
            stopped: Arc::new(Mutex::new(false)),
        }
    }

    /// Injects a gateway event (for testing or custom transport integration).
    pub fn inject_mock_event(&self, shard_id: u32, name: &str, payload_json: &str) {
        let _ = self.event_tx.send(ClientEventOutput {
            shard_id,
            name: name.to_string(),
            payload_json: payload_json.to_string(),
        });
    }

    /// Receives the next incoming Gateway event asynchronously.
    pub async fn next_event(&self) -> Option<ClientEventOutput> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    /// Sends a raw Gateway command (e.g. opcode 3, 4, 8) to a shard.
    pub fn send_command(&self, shard_id: u32, opcode: u8, payload_json: &str) -> Result<(), String> {
        self.command_tx
            .send(ShardCommand {
                shard_id,
                opcode,
                payload_json: payload_json.to_string(),
            })
            .map_err(|e| format!("Failed to send command: {e}"))
    }

    /// Sends an Update Presence command (opcode 3) to the specified shard.
    pub fn update_presence(
        &self,
        shard_id: u32,
        status: &str,
        activity_name: Option<&str>,
        activity_type: Option<u8>,
    ) -> Result<(), String> {
        let payload = serde_json::json!({
            "status": status,
            "since": null,
            "afk": false,
            "activities": activity_name.map(|name| {
                vec![serde_json::json!({
                    "name": name,
                    "type": activity_type.unwrap_or(0),
                })]
            }).unwrap_or_default(),
        });
        self.send_command(shard_id, 3, &payload.to_string())
    }

    /// Signals the client to stop.
    pub async fn stop(&self) {
        let mut stopped = self.stopped.lock().await;
        *stopped = true;
    }

    /// Returns the configured intents bitfield.
    pub fn intents(&self) -> u32 {
        self.intents
    }

    /// Returns the configured capabilities bitfield.
    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_client_event_dispatch_and_presence() {
        let client = NodeRiftClient::new("TEST_TOKEN".to_string(), Some(1), Some(0));
        assert_eq!(client.intents(), 1);
        assert_eq!(client.capabilities(), 0);

        // Inject event
        client.inject_mock_event(0, "MESSAGE_CREATE", r#"{"content":"hello"}"#);

        // Receive event
        let evt = client.next_event().await.expect("event received");
        assert_eq!(evt.shard_id, 0);
        assert_eq!(evt.name, "MESSAGE_CREATE");
        assert!(evt.payload_json.contains("hello"));

        // Update presence
        assert!(client.update_presence(0, "online", Some("Rift Bot"), Some(0)).is_ok());

        let cmd = client.command_rx.lock().await.recv().await.expect("command sent");
        assert_eq!(cmd.shard_id, 0);
        assert_eq!(cmd.opcode, 3);
        assert!(cmd.payload_json.contains("Rift Bot"));
    }
}
