use std::collections::VecDeque;

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

pub struct PaperExchange {
    bids: VecDeque<PaperOrder>,
    asks: VecDeque<PaperOrder>,
    last_price: f64,
}

impl PaperExchange {
    pub fn new() -> Self {
        Self {
            bids: VecDeque::new(),
            asks: VecDeque::new(),
            last_price: 0.0,
        }
    }

    pub fn on_tick(&mut self, price: f64, quantity: f64, is_buyer_maker: bool) -> Vec<PaperFill> {
        self.last_price = price;
        let mut fills = Vec::new();
        let mut remaining_qty = quantity;

        if is_buyer_maker {
            while let Some(bid) = self.bids.front_mut() {
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
                    self.bids.pop_front();
                }
                if remaining_qty <= 0.0 {
                    break;
                }
            }
        } else {
            while let Some(ask) = self.asks.front_mut() {
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
                    self.asks.pop_front();
                }
                if remaining_qty <= 0.0 {
                    break;
                }
            }
        }

        fills
    }

    pub fn submit_order(&mut self, order: PaperOrder) {
        if order.is_buy {
            let pos = self
                .bids
                .iter()
                .position(|o| o.price < order.price)
                .unwrap_or(self.bids.len());
            self.bids.insert(pos, order);
        } else {
            let pos = self
                .asks
                .iter()
                .position(|o| o.price > order.price)
                .unwrap_or(self.asks.len());
            self.asks.insert(pos, order);
        }
    }

    pub fn last_price(&self) -> f64 {
        self.last_price
    }
}
