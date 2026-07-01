use bytes::BytesMut;
use prost::Message;
use tokio::sync::mpsc;
use tracing::{debug, error};

use nemesis_core::EventEnvelope;

pub struct EventPublisher {
    tx: mpsc::Sender<Vec<u8>>,
}

impl EventPublisher {
    pub fn new(tx: mpsc::Sender<Vec<u8>>) -> Self {
        Self { tx }
    }

    pub async fn publish(&self, envelope: &EventEnvelope) -> anyhow::Result<()> {
        let mut buf = BytesMut::with_capacity(envelope.encoded_len());
        envelope.encode(&mut buf)?;

        if let Err(e) = self.tx.send(buf.to_vec()).await {
            error!("Failed to publish event: {}", e);
            return Err(anyhow::anyhow!("Channel closed"));
        }

        debug!(event_id = %envelope.event_id, "Published event");
        Ok(())
    }
}
