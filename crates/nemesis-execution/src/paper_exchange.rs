use std::collections::VecDeque;
use std::sync::Mutex;

use async_trait::async_trait;

use nemesis_core::OrderEvent;

use crate::exchange::{
    AccountBalance, Exchange, ExchangeError, ExchangePosition, NewOrder, OrderSide,
};

#[derive(Debug, Clone)]
pub struct PaperOrder {
    pub id: String,
    pub price: f64,
    pub quantity: f64,
    pub remaining: f64,
    pub is_buy: bool,
}

#[derive(Debug)]
pub struct PaperFill {
    pub order_id: String,
    pub price: f64,
    pub quantity: f64,
}

struct PaperExchangeInner {
    bids: VecDeque<PaperOrder>,
    asks: VecDeque<PaperOrder>,
    last_price: f64,
}

pub struct PaperExchange {
    inner: Mutex<PaperExchangeInner>,
}

impl PaperExchange {
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(PaperExchangeInner {
                bids: VecDeque::new(),
                asks: VecDeque::new(),
                last_price: 0.0,
            }),
        }
    }

    pub fn on_tick(&self, price: f64, quantity: f64, is_buyer_maker: bool) -> Vec<PaperFill> {
        let mut inner = self.inner.lock().unwrap();
        inner.last_price = price;
        let mut fills = Vec::new();
        let mut remaining_qty = quantity;

        if is_buyer_maker {
            while let Some(bid) = inner.bids.front_mut() {
                if price > bid.price {
                    break;
                }
                let fill_qty = remaining_qty.min(bid.remaining);
                fills.push(PaperFill {
                    order_id: bid.id.clone(),
                    price: bid.price,
                    quantity: fill_qty,
                });
                bid.remaining -= fill_qty;
                remaining_qty -= fill_qty;
                if bid.remaining <= 0.0 {
                    inner.bids.pop_front();
                }
                if remaining_qty <= 0.0 {
                    break;
                }
            }
        } else {
            while let Some(ask) = inner.asks.front_mut() {
                if price < ask.price {
                    break;
                }
                let fill_qty = remaining_qty.min(ask.remaining);
                fills.push(PaperFill {
                    order_id: ask.id.clone(),
                    price: ask.price,
                    quantity: fill_qty,
                });
                ask.remaining -= fill_qty;
                remaining_qty -= fill_qty;
                if ask.remaining <= 0.0 {
                    inner.asks.pop_front();
                }
                if remaining_qty <= 0.0 {
                    break;
                }
            }
        }

        fills
    }

    pub fn submit_order(&self, order: PaperOrder) {
        let mut inner = self.inner.lock().unwrap();
        if order.is_buy {
            let pos = inner
                .bids
                .iter()
                .position(|o| o.price < order.price)
                .unwrap_or(inner.bids.len());
            inner.bids.insert(pos, order);
        } else {
            let pos = inner
                .asks
                .iter()
                .position(|o| o.price > order.price)
                .unwrap_or(inner.asks.len());
            inner.asks.insert(pos, order);
        }
    }

    pub fn last_price(&self) -> f64 {
        self.inner.lock().unwrap().last_price
    }
}

#[async_trait]
impl Exchange for PaperExchange {
    async fn place_order(&self, order: &NewOrder) -> Result<String, ExchangeError> {
        let paper_order = PaperOrder {
            id: order.client_order_id.clone(),
            price: order.price.unwrap_or(0.0),
            quantity: order.quantity,
            remaining: order.quantity,
            is_buy: matches!(order.side, OrderSide::Buy),
        };
        self.submit_order(paper_order);
        Ok(order.client_order_id.clone())
    }

    async fn cancel_order(&self, _symbol: &str, client_order_id: &str) -> Result<(), ExchangeError> {
        let mut inner = self.inner.lock().unwrap();
        inner.bids.retain(|o| o.id != client_order_id);
        inner.asks.retain(|o| o.id != client_order_id);
        Ok(())
    }

    async fn get_balances(&self) -> Result<Vec<AccountBalance>, ExchangeError> {
        Ok(vec![
            AccountBalance {
                asset: "BTC".into(),
                free: 1.0,
                locked: 0.0,
            },
            AccountBalance {
                asset: "USDT".into(),
                free: 100000.0,
                locked: 0.0,
            },
        ])
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, ExchangeError> {
        Ok(vec![])
    }

    async fn get_open_orders(&self, _symbol: &str) -> Result<Vec<OrderEvent>, ExchangeError> {
        Ok(vec![])
    }

    async fn health_check(&self) -> Result<bool, ExchangeError> {
        Ok(true)
    }
}
