use std::sync::Arc;
use std::time::Duration;

use nemesis_core::proto::event_envelope::Payload;
use nemesis_core::{EventEnvelope, MetricsHandle, NoopMetrics, SessionState, SessionStateChange};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use crate::exchange::Exchange;

pub struct Reconciler<E: Exchange> {
    exchange: E,
    interval: Duration,
    tx: mpsc::Sender<EventEnvelope>,
    metrics: MetricsHandle,
}

impl<E: Exchange> Reconciler<E> {
    pub fn new(exchange: E, interval_secs: u64, tx: mpsc::Sender<EventEnvelope>) -> Self {
        Self {
            exchange,
            interval: Duration::from_secs(interval_secs),
            tx,
            metrics: Arc::new(NoopMetrics),
        }
    }

    pub fn with_metrics(mut self, metrics: MetricsHandle) -> Self {
        self.metrics = metrics;
        self
    }

    pub async fn run(&self) {
        loop {
            tokio::time::sleep(self.interval).await;

            match self.exchange.health_check().await {
                Ok(true) => {
                    info!("Reconciliation: exchange healthy");
                }
                Ok(false) => {
                    warn!("Reconciliation: exchange unhealthy");
                    self.metrics.record_reconciliation_drift("health_check_unhealthy");
                    self.emit_session_change(SessionState::StaleFeed, "Health check failed".into())
                        .await;
                }
                Err(e) => {
                    error!(?e, "Reconciliation: exchange error");
                    self.metrics.record_reconciliation_drift("exchange_error");
                    self.emit_session_change(
                        SessionState::Disconnected,
                        e.to_string(),
                    )
                    .await;
                }
            }
        }
    }

    async fn emit_session_change(&self, state: SessionState, reason: String) {
        let now_us = chrono::Utc::now().timestamp_micros();
        let envelope = EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            source: "reconciler".to_string(),
            symbol: "*".to_string(),
            exchange_ts_us: now_us,
            receive_ts_us: now_us,
            sequence_num: 0,
            payload: Some(Payload::Session(SessionStateChange {
                new_state: state as i32,
                reason,
            })),
        };
        let _ = self.tx.send(envelope).await;
    }
}
