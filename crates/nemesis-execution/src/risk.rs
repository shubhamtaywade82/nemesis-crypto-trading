use std::sync::Arc;

use nemesis_core::{MetricsHandle, NoopMetrics};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RiskViolation {
    #[error("Insufficient margin: required {required}, available {available}")]
    InsufficientMargin { required: f64, available: f64 },

    #[error("Position limit exceeded: current {current}, max {max}")]
    PositionLimitExceeded { current: f64, max: f64 },

    #[error("Max daily loss breached: {loss} > {limit}")]
    DailyLossBreached { loss: f64, limit: f64 },

    #[error("Spread too wide: {spread} > {max_spread}")]
    SpreadTooWide { spread: f64, max_spread: f64 },

    #[error("System halted: {reason}")]
    SystemHalted { reason: String },
}

pub struct RiskConfig {
    pub max_position_size: f64,
    pub max_daily_loss: f64,
    pub max_spread_bps: f64,
}

pub struct RiskEngine {
    config: RiskConfig,
    is_halted: bool,
    halt_reason: Option<String>,
    metrics: MetricsHandle,
}

impl RiskEngine {
    pub fn new(config: RiskConfig) -> Self {
        Self {
            config,
            is_halted: false,
            halt_reason: None,
            metrics: Arc::new(NoopMetrics),
        }
    }

    pub fn with_metrics(mut self, metrics: MetricsHandle) -> Self {
        self.metrics = metrics;
        self
    }

    pub fn validate(&self) -> Result<(), RiskViolation> {
        if self.is_halted {
            return Err(RiskViolation::SystemHalted {
                reason: self.halt_reason.clone().unwrap_or_default(),
            });
        }
        Ok(())
    }

    pub fn trigger_kill_switch(&mut self, reason: impl Into<String>) {
        self.is_halted = true;
        self.halt_reason = Some(reason.into());
        self.metrics.record_risk_violation("kill_switch");
        self.metrics.set_kill_switch(true);
        tracing::error!(reason = ?self.halt_reason, "KILL SWITCH ACTIVATED");
    }

    pub fn config(&self) -> &RiskConfig {
        &self.config
    }

    pub fn is_halted(&self) -> bool {
        self.is_halted
    }
}
