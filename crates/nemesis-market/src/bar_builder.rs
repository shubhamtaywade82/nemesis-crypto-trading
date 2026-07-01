use std::sync::Arc;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use nemesis_core::proto::event_envelope::Payload;
use nemesis_core::{BarClosed, EventEnvelope, MarketTick, MetricsHandle, NoopMetrics};
use uuid::Uuid;

/// Strict configuration for bar construction
#[derive(Debug, Clone)]
pub enum BarConfig {
    /// Time-based: closes exactly at interval boundary
    TimeBased { interval_secs: u64 },
    /// Volume-based: closes when cumulative volume >= threshold
    VolumeBased { threshold: f64 },
}

/// Tracks the state of a single forming bar
#[derive(Debug)]
struct FormingBar {
    open: f64,
    high: f64,
    low: f64,
    close: f64,
    volume: f64,
    buy_volume: f64,
    sell_volume: f64,
    first_seq: u64,
    last_seq: u64,
    start_exchange_ts: i64,
}

impl FormingBar {
    fn new(tick: &MarketTick, seq: u64, exchange_ts: i64) -> Self {
        let (buy_vol, sell_vol) = if tick.is_buyer_maker {
            (0.0, tick.quantity)
        } else {
            (tick.quantity, 0.0)
        };

        Self {
            open: tick.price,
            high: tick.price,
            low: tick.price,
            close: tick.price,
            volume: tick.quantity,
            buy_volume: buy_vol,
            sell_volume: sell_vol,
            first_seq: seq,
            last_seq: seq,
            start_exchange_ts: exchange_ts,
        }
    }

    fn update(&mut self, tick: &MarketTick, seq: u64) {
        self.high = self.high.max(tick.price);
        self.low = self.low.min(tick.price);
        self.close = tick.price;
        self.volume += tick.quantity;
        self.last_seq = seq;

        if tick.is_buyer_maker {
            self.sell_volume += tick.quantity;
        } else {
            self.buy_volume += tick.quantity;
        }
    }

    fn into_closed(self, bar_type: i32, is_corrupted: bool) -> BarClosed {
        BarClosed {
            r#type: bar_type,
            open: self.open,
            high: self.high,
            low: self.low,
            close: self.close,
            volume: self.volume,
            buy_volume: self.buy_volume,
            sell_volume: self.sell_volume,
            first_seq: self.first_seq,
            last_seq: self.last_seq,
            is_corrupted,
        }
    }
}

/// Deterministic bar builder with gap detection
pub struct BarBuilder {
    config: BarConfig,
    symbol: String,
    source: String,
    forming: Option<FormingBar>,
    expected_next_seq: Option<u64>,
    is_stale: bool,
    metrics: MetricsHandle,
}

impl BarBuilder {
    pub fn new(symbol: String, source: String, config: BarConfig) -> Self {
        Self {
            config,
            symbol,
            source,
            forming: None,
            expected_next_seq: None,
            is_stale: false,
            metrics: Arc::new(NoopMetrics),
        }
    }

    pub fn with_metrics(mut self, metrics: MetricsHandle) -> Self {
        self.metrics = metrics;
        self
    }

    fn bar_type_label(&self) -> &'static str {
        match &self.config {
            BarConfig::TimeBased { .. } => "time_1m",
            BarConfig::VolumeBased { .. } => "volume_100k",
        }
    }

    /// Process a raw tick and optionally emit a closed bar
    pub fn on_tick(
        &mut self,
        tick: &MarketTick,
        seq: u64,
        exchange_ts: i64,
    ) -> Option<EventEnvelope> {
        let start = Instant::now();

        // Gap detection: if we missed sequences, mark as corrupted
        if let Some(expected) = self.expected_next_seq {
            if seq != expected {
                tracing::warn!(
                    symbol = %self.symbol,
                    expected,
                    actual = seq,
                    "Sequence gap detected - marking bar as corrupted"
                );
                self.is_stale = true;
            }
        }
        self.expected_next_seq = Some(seq + 1);

        // Initialize or update forming bar
        if self.forming.is_none() {
            self.forming = Some(FormingBar::new(tick, seq, exchange_ts));
            return None;
        }

        let bar = self.forming.as_mut().unwrap();
        bar.update(tick, seq);

        // Check closure condition based on config
        let should_close = match &self.config {
            BarConfig::VolumeBased { threshold } => bar.volume >= *threshold,
            BarConfig::TimeBased { interval_secs } => {
                let elapsed = (exchange_ts - bar.start_exchange_ts) / 1_000_000;
                elapsed >= *interval_secs as i64
            }
        };

        if !should_close {
            return None;
        }

        let bar_type = match &self.config {
            BarConfig::TimeBased { .. } => 0,   // TIME_1M
            BarConfig::VolumeBased { .. } => 1, // VOLUME_100K_USDT
        };

        let is_corrupted = self.is_stale;
        let closed = self
            .forming
            .take()
            .unwrap()
            .into_closed(bar_type, is_corrupted);
        self.is_stale = false;

        let now_us = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_micros() as i64;

        let latency = start.elapsed().as_micros() as f64;
        let bar_type_label = self.bar_type_label();
        self.metrics
            .record_bar_closed(&self.symbol, bar_type_label, is_corrupted, latency);

        Some(EventEnvelope {
            event_id: Uuid::now_v7().to_string(),
            source: format!("{}-bar-builder", self.source),
            symbol: self.symbol.clone(),
            exchange_ts_us: exchange_ts,
            receive_ts_us: now_us,
            sequence_num: seq,
            payload: Some(Payload::Bar(closed)),
        })
    }

    /// Force-close current bar on session change or stale feed
    pub fn force_close(&mut self, reason: &str) -> Option<EventEnvelope> {
        if let Some(bar) = self.forming.take() {
            tracing::info!(symbol = %self.symbol, reason, "Force-closing bar");

            let bar_type = match &self.config {
                BarConfig::TimeBased { .. } => 0,
                BarConfig::VolumeBased { .. } => 1,
            };

            let closed = bar.into_closed(bar_type, true); // Always corrupted on force-close
            self.is_stale = false;

            let now_us = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_micros() as i64;

            let bar_type_label = self.bar_type_label();
            self.metrics
                .record_bar_forced_close(&self.symbol, bar_type_label);

            Some(EventEnvelope {
                event_id: Uuid::now_v7().to_string(),
                source: format!("{}-bar-builder", self.source),
                symbol: self.symbol.clone(),
                exchange_ts_us: now_us,
                receive_ts_us: now_us,
                sequence_num: 0,
                payload: Some(Payload::Bar(closed)),
            })
        } else {
            None
        }
    }
}
