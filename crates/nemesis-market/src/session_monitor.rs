use nemesis_core::proto::event_envelope::Payload;
use nemesis_core::{EventEnvelope, SessionState, SessionStateChange};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::warn;
use uuid::Uuid;

pub struct SessionMonitor {
    symbol: String,
    source: String,
    last_heartbeat: Instant,
    timeout: Duration,
    expected_seq: Option<u64>,
    tx: mpsc::Sender<EventEnvelope>,
}

impl SessionMonitor {
    pub fn new(
        symbol: String,
        source: String,
        timeout_secs: u64,
        tx: mpsc::Sender<EventEnvelope>,
    ) -> Self {
        Self {
            symbol,
            source,
            last_heartbeat: Instant::now(),
            timeout: Duration::from_secs(timeout_secs),
            expected_seq: None,
            tx,
        }
    }

    pub fn on_tick(&mut self, seq: u64) {
        self.last_heartbeat = Instant::now();

        if let Some(expected) = self.expected_seq {
            if seq != expected {
                warn!(
                    symbol = %self.symbol,
                    expected,
                    actual = seq,
                    "Sequence gap detected"
                );
                self.emit_state_change(
                    SessionState::StaleFeed,
                    "Sequence gap".to_string(),
                );
            }
        }
        self.expected_seq = Some(seq + 1);
    }

    pub async fn monitor_loop(&mut self) {
        loop {
            tokio::time::sleep(Duration::from_secs(1)).await;
            if self.last_heartbeat.elapsed() > self.timeout {
                warn!(
                    symbol = %self.symbol,
                    "Heartbeat timeout - feed stale"
                );
                self.emit_state_change(
                    SessionState::StaleFeed,
                    "Heartbeat timeout".to_string(),
                );
            }
        }
    }

    fn emit_state_change(&self, state: SessionState, reason: String) {
        let now_us = chrono::Utc::now().timestamp_micros();
        let envelope = EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            source: format!("{}-session-monitor", self.source),
            symbol: self.symbol.clone(),
            exchange_ts_us: now_us,
            receive_ts_us: now_us,
            sequence_num: 0,
            payload: Some(Payload::Session(SessionStateChange {
                new_state: state as i32,
                reason,
            })),
        };

        if let Err(e) = self.tx.try_send(envelope) {
            warn!("Failed to send session state change: {}", e);
        }
    }
}
