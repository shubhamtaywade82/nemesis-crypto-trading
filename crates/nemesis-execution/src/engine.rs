use std::sync::Arc;

use nemesis_core::{EventEnvelope, MetricsHandle, NoopMetrics};
use tokio::sync::mpsc;
use tracing::{info, warn};

use crate::paper_exchange::PaperExchange;
use crate::risk::{RiskConfig, RiskEngine};

#[allow(dead_code)]
pub struct ExecutionEngine {
    risk: RiskEngine,
    paper: PaperExchange,
    tx: mpsc::Sender<EventEnvelope>,
    metrics: MetricsHandle,
}

impl ExecutionEngine {
    pub fn new(risk_config: RiskConfig, tx: mpsc::Sender<EventEnvelope>) -> Self {
        let metrics = Arc::new(NoopMetrics);
        Self {
            risk: RiskEngine::new(risk_config).with_metrics(metrics.clone()),
            paper: PaperExchange::new(),
            tx,
            metrics,
        }
    }

    pub fn with_metrics(
        risk_config: RiskConfig,
        tx: mpsc::Sender<EventEnvelope>,
        metrics: MetricsHandle,
    ) -> Self {
        Self {
            risk: RiskEngine::new(risk_config).with_metrics(metrics.clone()),
            paper: PaperExchange::new(),
            tx,
            metrics,
        }
    }

    pub async fn on_signal(&mut self, envelope: &EventEnvelope) {
        if let Err(violation) = self.risk.validate() {
            warn!(?violation, "Signal rejected by risk engine");
            self.metrics
                .record_order_rejected(&envelope.symbol, &violation.to_string());
            return;
        }

        let side = match envelope.payload.as_ref().and_then(|p| {
            if let nemesis_core::proto::event_envelope::Payload::Signal(ref s) = p {
                Some(s.side)
            } else {
                None
            }
        }) {
            Some(0) => "buy",
            Some(1) => "sell",
            _ => "unknown",
        };

        self.metrics.record_signal_received(&envelope.symbol, side);
        self.metrics
            .record_order_submitted(&envelope.symbol, side, "market");

        info!("Signal approved, submitting to paper exchange");
    }

    pub fn trigger_kill_switch(&mut self, reason: impl Into<String>) {
        self.risk.trigger_kill_switch(reason);
    }

    pub fn is_halted(&self) -> bool {
        self.risk.is_halted()
    }
}
