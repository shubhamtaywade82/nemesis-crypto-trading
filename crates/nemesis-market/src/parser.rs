use nemesis_core::MarketTick;
use serde::Deserialize;

/// Binance raw trade message from @aggTrade stream
#[derive(Debug, Deserialize)]
pub struct BinanceAggTrade {
    #[serde(rename = "a")]
    pub agg_trade_id: u64,
    #[serde(rename = "p")]
    pub price: String,
    #[serde(rename = "q")]
    pub quantity: String,
    #[serde(rename = "T")]
    pub timestamp: i64,
    #[serde(rename = "m")]
    pub is_buyer_maker: bool,
}

impl BinanceAggTrade {
    pub fn to_market_tick(&self) -> anyhow::Result<MarketTick> {
        Ok(MarketTick {
            price: self.price.parse::<f64>()?,
            quantity: self.quantity.parse::<f64>()?,
            is_buyer_maker: self.is_buyer_maker,
        })
    }
}
