use axum::{routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

use crate::metrics::NemesisMetrics;

pub struct HttpServer {
    metrics: Arc<NemesisMetrics>,
}

impl HttpServer {
    pub fn new(metrics: Arc<NemesisMetrics>) -> Self {
        Self { metrics }
    }

    pub async fn run(self, addr: &str) -> anyhow::Result<()> {
        let metrics = self.metrics;

        let app = Router::new()
            .route("/health", get(move || async {
                Json(json!({
                    "status": "healthy",
                    "version": env!("CARGO_PKG_VERSION"),
                    "timestamp": chrono::Utc::now().to_rfc3339()
                }))
            }))
            .route("/metrics", get(move || async {
                metrics.encode()
            }));

        tracing::info!(addr = %addr, "HTTP server starting");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
