use async_trait::async_trait;
use hmac::{Hmac, Mac};
use reqwest::Client;
use sha2::Sha256;
use tracing::debug;

use super::{AccountBalance, Exchange, ExchangeError, ExchangePosition, NewOrder};
use crate::rate_limiter::RateLimiter;
use nemesis_core::OrderEvent;

type HmacSha256 = Hmac<Sha256>;

pub struct BinanceFutures {
    client: Client,
    api_key: String,
    api_secret: String,
    base_url: String,
    rate_limiter: RateLimiter,
}

impl BinanceFutures {
    pub fn new(api_key: String, api_secret: String, use_testnet: bool) -> Self {
        let base_url = if use_testnet {
            "https://testnet.binancefuture.com"
        } else {
            "https://fapi.binance.com"
        }
        .to_string();

        Self {
            client: Client::new(),
            api_key,
            api_secret,
            base_url,
            rate_limiter: RateLimiter::new(40, 40),
        }
    }

    fn sign(&self, query_string: &str) -> String {
        let mut mac =
            HmacSha256::new_from_slice(self.api_secret.as_bytes()).expect("HMAC key error");
        mac.update(query_string.as_bytes());
        hex::encode(mac.finalize().into_bytes())
    }

    async fn signed_get(
        &self,
        path: &str,
        params: &[(&str, &str)],
    ) -> Result<serde_json::Value, ExchangeError> {
        self.rate_limiter.acquire().await;

        let timestamp = chrono::Utc::now().timestamp_millis().to_string();
        let mut query_parts: Vec<String> =
            params.iter().map(|(k, v)| format!("{}={}", k, v)).collect();
        query_parts.push(format!("timestamp={}", timestamp));

        let query_string = query_parts.join("&");
        let signature = self.sign(&query_string);
        let url = format!(
            "{}/{}?{}&signature={}",
            self.base_url, path, query_string, signature
        );

        debug!(url = %url, "Signed GET request");

        let resp = self
            .client
            .get(&url)
            .header("X-MBX-APIKEY", &self.api_key)
            .send()
            .await?;

        if resp.status() == 429 {
            let retry_after = resp
                .headers()
                .get("Retry-After")
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse().ok())
                .unwrap_or(1000);
            return Err(ExchangeError::RateLimited {
                retry_after_ms: retry_after,
            });
        }

        let body: serde_json::Value = resp.json().await?;
        Ok(body)
    }
}

#[async_trait]
impl Exchange for BinanceFutures {
    async fn place_order(&self, _order: &NewOrder) -> Result<String, ExchangeError> {
        todo!("Implement Binance order placement")
    }

    async fn cancel_order(
        &self,
        _symbol: &str,
        _client_order_id: &str,
    ) -> Result<(), ExchangeError> {
        todo!("Implement Binance order cancellation")
    }

    async fn get_balances(&self) -> Result<Vec<AccountBalance>, ExchangeError> {
        let _body = self.signed_get("/fapi/v2/balance", &[]).await?;
        todo!("Parse Binance balance response")
    }

    async fn get_positions(&self) -> Result<Vec<ExchangePosition>, ExchangeError> {
        let _body = self.signed_get("/fapi/v2/positionRisk", &[]).await?;
        todo!("Parse Binance position response")
    }

    async fn get_open_orders(&self, _symbol: &str) -> Result<Vec<OrderEvent>, ExchangeError> {
        let _body = self
            .signed_get("/fapi/v1/openOrders", &[("symbol", _symbol)])
            .await?;
        todo!("Parse Binance open orders response")
    }

    async fn health_check(&self) -> Result<bool, ExchangeError> {
        self.rate_limiter.acquire().await;
        let url = format!("{}/fapi/v1/ping", self.base_url);
        let resp = self.client.get(&url).send().await?;
        Ok(resp.status().is_success())
    }
}
