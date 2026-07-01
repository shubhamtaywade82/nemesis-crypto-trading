pub mod binance;

use async_trait::async_trait;
use nemesis_core::OrderEvent;

#[derive(Debug, Clone)]
pub struct NewOrder {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<f64>,
    pub quantity: f64,
    pub client_order_id: String,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy)]
pub enum OrderType {
    Limit,
    Market,
}

#[derive(Debug, Clone)]
pub struct AccountBalance {
    pub asset: String,
    pub free: f64,
    pub locked: f64,
}

#[derive(Debug, Clone)]
pub struct ExchangePosition {
    pub symbol: String,
    pub position_amt: f64,
    pub entry_price: f64,
    pub unrealized_pnl: f64,
}

#[async_trait]
pub trait Exchange: Send + Sync {
    async fn place_order(&self, order: &NewOrder) -> Result<String, ExchangeError>;

    async fn cancel_order(&self, symbol: &str, client_order_id: &str) -> Result<(), ExchangeError>;

    async fn get_balances(&self) -> Result<Vec<AccountBalance>, ExchangeError>;

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, ExchangeError>;

    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<OrderEvent>, ExchangeError>;

    async fn health_check(&self) -> Result<bool, ExchangeError>;
}

#[async_trait]
impl<T: Exchange + ?Sized> Exchange for Box<T> {
    async fn place_order(&self, order: &NewOrder) -> Result<String, ExchangeError> {
        (**self).place_order(order).await
    }

    async fn cancel_order(&self, symbol: &str, client_order_id: &str) -> Result<(), ExchangeError> {
        (**self).cancel_order(symbol, client_order_id).await
    }

    async fn get_balances(&self) -> Result<Vec<AccountBalance>, ExchangeError> {
        (**self).get_balances().await
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, ExchangeError> {
        (**self).get_positions().await
    }

    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<OrderEvent>, ExchangeError> {
        (**self).get_open_orders(symbol).await
    }

    async fn health_check(&self) -> Result<bool, ExchangeError> {
        (**self).health_check().await
    }
}

#[derive(thiserror::Error, Debug)]
pub enum ExchangeError {
    #[error("Authentication failed: {0}")]
    Auth(String),
    #[error("Rate limited: retry after {retry_after_ms}ms")]
    RateLimited { retry_after_ms: u64 },
    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),
    #[error("Exchange rejected order: {code} - {msg}")]
    Rejected { code: i32, msg: String },
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}
