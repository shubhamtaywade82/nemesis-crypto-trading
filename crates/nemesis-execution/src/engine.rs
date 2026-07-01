use tokio::sync::mpsc;
use tracing::{info, warn};
use nemesis_core::EventEnvelope;
use crate::paper_exchange::PaperExchange;
use crate::risk::{RiskConfig, RiskEngine};

pub struct ExecutionEngine {
    risk: RiskEngine,
    paper: PaperExchange,
    tx: mpsc::Sender<EventEnvelope>,
}

impl ExecutionEngine {
    pub fn new(risk_config: RiskConfig, tx: mpsc::Sender<EventEnvelope>) -> Self {
        Self {
            risk: RiskEngine::new(risk_config),
            paper: PaperExchange::new(),
            tx,
        }
    }

    pub async fn on_signal(&mut self, _envelope: &EventEnvelope) {
        if let Err(violation) = self.risk.validate() {
            warn!(?violation, "Signal rejected by risk engine");
            return;
        }

        info!("Signal approved, submitting to paper exchange");
    }

    pub fn trigger_kill_switch(&mut self, reason: impl Into<String>) {
        self.risk.trigger_kill_switch(reason);
    }

    pub fn is_halted(&self) -> bool {
        self.risk.is_halted()
    }
}
