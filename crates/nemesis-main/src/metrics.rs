use prometheus::{
    Encoder, HistogramOpts, HistogramVec, IntCounterVec, IntGaugeVec, Opts, Registry, TextEncoder,
};
use std::sync::Arc;

#[derive(Clone)]
pub struct NemesisMetrics {
    pub registry: Arc<Registry>,
    pub bars_processed: IntCounterVec,
    pub bars_corrupted: IntCounterVec,
    pub signals_received: IntCounterVec,
    pub orders_submitted: IntCounterVec,
    pub orders_rejected: IntCounterVec,
    pub risk_violations: IntCounterVec,
    pub ws_reconnections: IntCounterVec,
    pub reconciliation_drift: IntCounterVec,
    pub bar_build_latency_us: HistogramVec,
    #[allow(dead_code)]
    pub active_positions: IntGaugeVec,
    pub kill_switch_active: IntGaugeVec,
}

impl NemesisMetrics {
    pub fn new() -> Self {
        let registry = Registry::new();

        let bars_processed = IntCounterVec::new(
            Opts::new("nemesis_bars_processed_total", "Total bars processed"),
            &["symbol", "bar_type"],
        )
        .unwrap();

        let bars_corrupted = IntCounterVec::new(
            Opts::new(
                "nemesis_bars_corrupted_total",
                "Total corrupted bars detected",
            ),
            &["symbol"],
        )
        .unwrap();

        let signals_received = IntCounterVec::new(
            Opts::new(
                "nemesis_signals_received_total",
                "Trade signals received from Python",
            ),
            &["symbol", "side"],
        )
        .unwrap();

        let orders_submitted = IntCounterVec::new(
            Opts::new(
                "nemesis_orders_submitted_total",
                "Orders submitted to exchange",
            ),
            &["symbol", "side", "order_type"],
        )
        .unwrap();

        let orders_rejected = IntCounterVec::new(
            Opts::new(
                "nemesis_orders_rejected_total",
                "Orders rejected by risk engine",
            ),
            &["symbol", "reason"],
        )
        .unwrap();

        let risk_violations = IntCounterVec::new(
            Opts::new("nemesis_risk_violations_total", "Risk violations triggered"),
            &["violation_type"],
        )
        .unwrap();

        let ws_reconnections = IntCounterVec::new(
            Opts::new(
                "nemesis_ws_reconnections_total",
                "WebSocket reconnection attempts",
            ),
            &["symbol"],
        )
        .unwrap();

        let reconciliation_drift = IntCounterVec::new(
            Opts::new(
                "nemesis_reconciliation_drift_total",
                "Reconciliation drift events",
            ),
            &["drift_type"],
        )
        .unwrap();

        let bar_build_latency_us = HistogramVec::new(
            HistogramOpts::new(
                "nemesis_bar_build_latency_us",
                "Bar build latency in microseconds",
            )
            .buckets(vec![10.0, 50.0, 100.0, 500.0, 1000.0, 5000.0]),
            &["symbol"],
        )
        .unwrap();

        let active_positions = IntGaugeVec::new(
            Opts::new("nemesis_active_positions", "Currently active positions"),
            &["symbol", "side"],
        )
        .unwrap();

        let kill_switch_active = IntGaugeVec::new(
            Opts::new(
                "nemesis_kill_switch_active",
                "Kill switch status (1=active, 0=normal)",
            ),
            &[],
        )
        .unwrap();

        registry.register(Box::new(bars_processed.clone())).unwrap();
        registry.register(Box::new(bars_corrupted.clone())).unwrap();
        registry
            .register(Box::new(signals_received.clone()))
            .unwrap();
        registry
            .register(Box::new(orders_submitted.clone()))
            .unwrap();
        registry
            .register(Box::new(orders_rejected.clone()))
            .unwrap();
        registry
            .register(Box::new(risk_violations.clone()))
            .unwrap();
        registry
            .register(Box::new(ws_reconnections.clone()))
            .unwrap();
        registry
            .register(Box::new(reconciliation_drift.clone()))
            .unwrap();
        registry
            .register(Box::new(bar_build_latency_us.clone()))
            .unwrap();
        registry
            .register(Box::new(active_positions.clone()))
            .unwrap();
        registry
            .register(Box::new(kill_switch_active.clone()))
            .unwrap();

        Self {
            registry: Arc::new(registry),
            bars_processed,
            bars_corrupted,
            signals_received,
            orders_submitted,
            orders_rejected,
            risk_violations,
            ws_reconnections,
            reconciliation_drift,
            bar_build_latency_us,
            active_positions,
            kill_switch_active,
        }
    }

    pub fn encode(&self) -> String {
        let encoder = TextEncoder::new();
        let metric_families = self.registry.gather();
        let mut buffer = Vec::new();
        encoder.encode(&metric_families, &mut buffer).unwrap();
        String::from_utf8(buffer).unwrap()
    }
}

use nemesis_core::MetricsRecorder;

impl MetricsRecorder for NemesisMetrics {
    fn record_bar_closed(&self, symbol: &str, bar_type: &str, corrupted: bool, latency_us: f64) {
        self.bars_processed
            .with_label_values(&[symbol, bar_type])
            .inc();
        if corrupted {
            self.bars_corrupted.with_label_values(&[symbol]).inc();
        }
        self.bar_build_latency_us
            .with_label_values(&[symbol])
            .observe(latency_us);
    }

    fn record_bar_forced_close(&self, symbol: &str, bar_type: &str) {
        self.bars_processed
            .with_label_values(&[symbol, bar_type])
            .inc();
        self.bars_corrupted.with_label_values(&[symbol]).inc();
    }

    fn record_ws_reconnection(&self, symbol: &str) {
        self.ws_reconnections.with_label_values(&[symbol]).inc();
    }

    fn record_signal_received(&self, symbol: &str, side: &str) {
        self.signals_received
            .with_label_values(&[symbol, side])
            .inc();
    }

    fn record_order_submitted(&self, symbol: &str, side: &str, order_type: &str) {
        self.orders_submitted
            .with_label_values(&[symbol, side, order_type])
            .inc();
    }

    fn record_order_rejected(&self, symbol: &str, reason: &str) {
        self.orders_rejected
            .with_label_values(&[symbol, reason])
            .inc();
    }

    fn record_risk_violation(&self, violation_type: &str) {
        self.risk_violations
            .with_label_values(&[violation_type])
            .inc();
    }

    fn record_reconciliation_drift(&self, drift_type: &str) {
        self.reconciliation_drift
            .with_label_values(&[drift_type])
            .inc();
    }

    fn set_kill_switch(&self, active: bool) {
        self.kill_switch_active
            .with_label_values(&[] as &[&str])
            .set(if active { 1 } else { 0 });
    }
}
