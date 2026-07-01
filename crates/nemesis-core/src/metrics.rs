use std::sync::Arc;

pub trait MetricsRecorder: Send + Sync {
    fn record_bar_closed(&self, symbol: &str, bar_type: &str, corrupted: bool, latency_us: f64);
    fn record_bar_forced_close(&self, symbol: &str, bar_type: &str);
    fn record_ws_reconnection(&self, symbol: &str);
    fn record_signal_received(&self, symbol: &str, side: &str);
    fn record_order_submitted(&self, symbol: &str, side: &str, order_type: &str);
    fn record_order_rejected(&self, symbol: &str, reason: &str);
    fn record_risk_violation(&self, violation_type: &str);
    fn record_reconciliation_drift(&self, drift_type: &str);
    fn set_kill_switch(&self, active: bool);
}

pub type MetricsHandle = Arc<dyn MetricsRecorder>;

pub struct NoopMetrics;

impl MetricsRecorder for NoopMetrics {
    fn record_bar_closed(
        &self,
        _symbol: &str,
        _bar_type: &str,
        _corrupted: bool,
        _latency_us: f64,
    ) {
    }
    fn record_bar_forced_close(&self, _symbol: &str, _bar_type: &str) {}
    fn record_ws_reconnection(&self, _symbol: &str) {}
    fn record_signal_received(&self, _symbol: &str, _side: &str) {}
    fn record_order_submitted(&self, _symbol: &str, _side: &str, _order_type: &str) {}
    fn record_order_rejected(&self, _symbol: &str, _reason: &str) {}
    fn record_risk_violation(&self, _violation_type: &str) {}
    fn record_reconciliation_drift(&self, _drift_type: &str) {}
    fn set_kill_switch(&self, _active: bool) {}
}
