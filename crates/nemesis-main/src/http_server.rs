use axum::{routing::get, Json, Router};
use serde_json::json;
use std::sync::Arc;

const NEMESIS_VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::metrics::NemesisMetrics;

pub struct HttpServer {
    metrics: Arc<NemesisMetrics>,
}

impl HttpServer {
    pub fn new(metrics: Arc<NemesisMetrics>) -> Self {
        Self { metrics }
    }

    pub async fn run(self, addr: &str) -> anyhow::Result<()> {
        let app = Router::new()
            .route(
                "/health",
                get(|| async {
                    Json(json!({
                        "status": "healthy",
                        "version": NEMESIS_VERSION,
                        "timestamp": chrono::Utc::now().to_rfc3339()
                    }))
                }),
            )
            .route("/metrics", {
                let metrics = self.metrics;
                get(|| async move { metrics.encode() })
            });

        tracing::info!(addr = %addr, "HTTP server starting");
        let listener = tokio::net::TcpListener::bind(addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}
