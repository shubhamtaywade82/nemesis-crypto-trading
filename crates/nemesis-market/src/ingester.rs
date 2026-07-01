use crate::bar_builder::{BarBuilder, BarConfig};
use crate::parser::BinanceAggTrade;
use crate::session_monitor::SessionMonitor;
use futures_util::{SinkExt, StreamExt};
use nemesis_core::EventEnvelope;
use tokio::sync::mpsc;
use tracing::{debug, error, info};
use std::time::{SystemTime, UNIX_EPOCH};

pub struct MarketIngester {
    symbol: String,
    ws_url: String,
    bar_builder: BarBuilder,
    tx: mpsc::Sender<EventEnvelope>,
}

impl MarketIngester {
    pub fn new(
        symbol: String,
        ws_url: String,
        bar_config: BarConfig,
        tx: mpsc::Sender<EventEnvelope>,
    ) -> Self {
        let source = "binance-ws".to_string();
        Self {
            symbol: symbol.clone(),
            ws_url,
            bar_builder: BarBuilder::new(symbol.clone(), source.clone(), bar_config),
            tx,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        info!(
            symbol = %self.symbol,
            url = %self.ws_url,
            "Connecting to WebSocket"
        );

        let (ws_stream, _) = tokio_tungstenite::connect_async(&self.ws_url).await?;
        let (mut write, mut read) = ws_stream.split();

        let subscribe_msg = format!(
            r#"{{"method": "SUBSCRIBE", "params": ["{}@aggTrade"], "id": 1}}"#,
            self.symbol.to_lowercase()
        );
        write
            .send(tokio_tungstenite::tungstenite::Message::Text(subscribe_msg))
            .await?;

        // Spawn session monitor for this symbol
        let source = "binance-ws".to_string();
        let mut monitor = SessionMonitor::new(
            self.symbol.clone(),
            source,
            10, // 10s heartbeat timeout
            self.tx.clone(),
        );
        tokio::spawn(async move {
            monitor.monitor_loop().await;
        });

        while let Some(msg) = read.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    // Binance aggTrade JSON may be wrapped in a stream envelope
                    if let Ok(trade) = serde_json::from_str::<BinanceAggTrade>(&text) {
                        let tick = trade.to_market_tick()?;
                        let seq = trade.agg_trade_id;
                        let ts_us = trade.timestamp * 1000; // ms to us

                        let now_us = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_micros() as i64;

                        // Process tick through bar builder first
                        if let Some(bar_event) = self.bar_builder.on_tick(&tick, seq, ts_us) {
                            if let Err(e) = self.tx.send(bar_event).await {
                                error!("Failed to send bar event: {}", e);
                            }
                        }

                        let envelope = EventEnvelope {
                            event_id: uuid::Uuid::now_v7().to_string(),
                            source: "binance-ws".to_string(),
                            symbol: self.symbol.clone(),
                            exchange_ts_us: ts_us,
                            receive_ts_us: now_us,
                            sequence_num: seq,
                            payload: Some(
                                nemesis_core::proto::event_envelope::Payload::Tick(tick),
                            ),
                        };

                        if let Err(e) = self.tx.send(envelope).await {
                            error!("Failed to send tick event: {}", e);
                        }
                    } else {
                        debug!("Non-trade message: {}", text);
                    }
                }
                Ok(tokio_tungstenite::tungstenite::Message::Ping(data)) => {
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Pong(data)).await {
                        error!("Failed to send pong: {}", e);
                    }
                }
                Ok(_) => {} // Ignore pong/binary
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
            }
        }

        Ok(())
    }
}
