pub mod metrics;

pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nemesis.v1.rs"));
}

pub use metrics::{MetricsHandle, MetricsRecorder, NoopMetrics};
pub use proto::{BarClosed, EventEnvelope, MarketTick, OrderEvent, SessionState, SessionStateChange, TradeSignal};
