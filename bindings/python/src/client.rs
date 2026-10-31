use rift_core::{Capabilities, Intents};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, Mutex};

/// Python async Rift Client wrapper.
pub struct PyRiftClient {
    #[allow(dead_code)]
    token: String,
    intents: u32,
    capabilities: u32,
    event_tx: mpsc::UnboundedSender<(u32, String, String)>,
    event_rx: Arc<Mutex<mpsc::UnboundedReceiver<(u32, String, String)>>>,
    command_tx: mpsc::UnboundedSender<(u32, u8, String)>,
    #[allow(dead_code)]
    command_rx: Arc<Mutex<mpsc::UnboundedReceiver<(u32, u8, String)>>>,
    stopped: Arc<Mutex<bool>>,
}

impl PyRiftClient {
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

    pub fn inject_mock_event(&self, shard_id: u32, name: &str, payload_json: &str) {
        let _ = self.event_tx.send((shard_id, name.to_string(), payload_json.to_string()));
    }

    pub fn poll_event(&self) -> Option<(u32, String, String)> {
        let mut rx = self.event_rx.try_lock().ok()?;
        rx.try_recv().ok()
    }

    pub async fn next_event(&self) -> Option<(u32, String, String)> {
        let mut rx = self.event_rx.lock().await;
        rx.recv().await
    }

    pub fn recv_timeout(&self, timeout_ms: u64) -> Option<(u32, String, String)> {
        let rx = self.event_rx.clone();
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(async {
                let mut lock = rx.lock().await;
                tokio::time::timeout(Duration::from_millis(timeout_ms), lock.recv())
                    .await
                    .ok()
                    .flatten()
            })
        })
    }

    pub fn send_command(&self, shard_id: u32, opcode: u8, payload_json: &str) -> Result<(), String> {
        self.command_tx
            .send((shard_id, opcode, payload_json.to_string()))
            .map_err(|e| format!("Failed to send command: {e}"))
    }

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

    pub async fn stop(&self) {
        let mut stopped = self.stopped.lock().await;
        *stopped = true;
    }

    pub fn is_stopped(&self) -> bool {
        self.stopped.try_lock().map(|s| *s).unwrap_or(false)
    }

    pub fn intents(&self) -> u32 {
        self.intents
    }

    pub fn capabilities(&self) -> u32 {
        self.capabilities
    }
}

#[cfg(feature = "pyo3-binding")]
pub mod pyo3_impl {
    use super::*;
    use pyo3::prelude::*;

    #[pyclass(name = "RiftClient")]
    pub struct PyRiftClientClass {
        inner: Arc<PyRiftClient>,
    }

    #[pymethods]
    impl PyRiftClientClass {
        #[new]
        #[pyo3(signature = (token, intents=None, capabilities=None))]
        pub fn new(token: String, intents: Option<u32>, capabilities: Option<u32>) -> Self {
            Self {
                inner: Arc::new(PyRiftClient::new(token, intents, capabilities)),
            }
        }

        pub fn inject_mock_event(&self, shard_id: u32, name: &str, payload_json: &str) {
            self.inner.inject_mock_event(shard_id, name, payload_json);
        }

        pub fn poll_event(&self) -> Option<(u32, String, String)> {
            self.inner.poll_event()
        }

        pub fn send_command(&self, shard_id: u32, opcode: u8, payload_json: &str) -> PyResult<()> {
            self.inner
                .send_command(shard_id, opcode, payload_json)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        #[pyo3(signature = (shard_id, status, activity_name=None, activity_type=None))]
        pub fn update_presence(
            &self,
            shard_id: u32,
            status: &str,
            activity_name: Option<&str>,
            activity_type: Option<u8>,
        ) -> PyResult<()> {
            self.inner
                .update_presence(shard_id, status, activity_name, activity_type)
                .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e))
        }

        pub fn stop(&self) {
            let stopped = self.inner.stopped.clone();
            if let Ok(mut s) = stopped.try_lock() {
                *s = true;
            }
        }

        pub fn is_stopped(&self) -> bool {
            self.inner.is_stopped()
        }

        pub fn intents(&self) -> u32 {
            self.inner.intents()
        }

        pub fn capabilities(&self) -> u32 {
            self.inner.capabilities()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_py_client() {
        let client = PyRiftClient::new("TOKEN".to_string(), Some(1), Some(0));
        client.inject_mock_event(0, "READY", "{}");
        let evt = client.next_event().await.unwrap();
        assert_eq!(evt.0, 0);
        assert_eq!(evt.1, "READY");
    }
}
