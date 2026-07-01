pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/nemesis.v1.rs"));
}

pub use proto::{BarClosed, EventEnvelope, MarketTick, SessionState, SessionStateChange};
