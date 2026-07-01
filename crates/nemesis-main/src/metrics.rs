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
            Opts::new("nemesis_bars_corrupted_total", "Total corrupted bars detected"),
            &["symbol"],
        )
        .unwrap();

        let signals_received = IntCounterVec::new(
            Opts::new("nemesis_signals_received_total", "Trade signals received from Python"),
            &["symbol", "side"],
        )
        .unwrap();

        let orders_submitted = IntCounterVec::new(
            Opts::new("nemesis_orders_submitted_total", "Orders submitted to exchange"),
            &["symbol", "side", "order_type"],
        )
        .unwrap();

        let orders_rejected = IntCounterVec::new(
            Opts::new("nemesis_orders_rejected_total", "Orders rejected by risk engine"),
            &["symbol", "reason"],
        )
        .unwrap();

        let risk_violations = IntCounterVec::new(
            Opts::new("nemesis_risk_violations_total", "Risk violations triggered"),
            &["violation_type"],
        )
        .unwrap();

        let ws_reconnections = IntCounterVec::new(
            Opts::new("nemesis_ws_reconnections_total", "WebSocket reconnection attempts"),
            &["symbol"],
        )
        .unwrap();

        let reconciliation_drift = IntCounterVec::new(
            Opts::new("nemesis_reconciliation_drift_total", "Reconciliation drift events"),
            &["drift_type"],
        )
        .unwrap();

        let bar_build_latency_us = HistogramVec::new(
            HistogramOpts::new("nemesis_bar_build_latency_us", "Bar build latency in microseconds")
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
            Opts::new("nemesis_kill_switch_active", "Kill switch status (1=active, 0=normal)"),
            &[],
        )
        .unwrap();

        registry.register(Box::new(bars_processed.clone())).unwrap();
        registry.register(Box::new(bars_corrupted.clone())).unwrap();
        registry.register(Box::new(signals_received.clone())).unwrap();
        registry.register(Box::new(orders_submitted.clone())).unwrap();
        registry.register(Box::new(orders_rejected.clone())).unwrap();
        registry.register(Box::new(risk_violations.clone())).unwrap();
        registry.register(Box::new(ws_reconnections.clone())).unwrap();
        registry.register(Box::new(reconciliation_drift.clone())).unwrap();
        registry.register(Box::new(bar_build_latency_us.clone())).unwrap();
        registry.register(Box::new(active_positions.clone())).unwrap();
        registry.register(Box::new(kill_switch_active.clone())).unwrap();

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
