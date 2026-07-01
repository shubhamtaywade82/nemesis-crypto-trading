mod config;

use anyhow::Result;
use tokio::signal;
use tokio::sync::mpsc;
use tracing::{error, info};
use tracing_subscriber::{fmt, EnvFilter};

use nemesis_core::EventEnvelope;
use nemesis_execution::{BinanceFutures, ExecutionEngine, Reconciler, RiskConfig as ExecRiskConfig};
use nemesis_market::{BarConfig, MarketIngester};

use crate::config::AppConfig;

#[tokio::main]
async fn main() -> Result<()> {
    let config_path =
        std::env::var("NEMESIS_CONFIG").unwrap_or_else(|_| "config/nemesis.toml".into());
    let config = AppConfig::load(&config_path)?;

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.logging.level));

    match config.logging.format.as_str() {
        "json" => fmt().json().with_env_filter(filter).init(),
        _ => fmt().pretty().with_env_filter(filter).init(),
    }

    info!(config = %config_path, "Nemesis starting");

    if !config.exchange.testnet {
        tracing::warn!("RUNNING ON MAINNET - Real funds at risk");
        tracing::warn!(
            "Max position: {} | Max daily loss: {}",
            config.risk.max_position_size,
            config.risk.max_daily_loss
        );

        if std::env::var("NEMESIS_MAINNET_CONFIRM").unwrap_or_default() != "YES" {
            anyhow::bail!(
                "Mainnet mode requires NEMESIS_MAINNET_CONFIRM=YES environment variable"
            );
        }
    }

    let (market_tx, mut market_rx) = mpsc::channel::<EventEnvelope>(10_000);
    let (exec_tx, _exec_rx) = mpsc::channel::<EventEnvelope>(1_000);

    let mut ingest_handles = Vec::new();
    for sym_cfg in &config.symbols {
        let symbol = sym_cfg.symbol.clone();
        let ws_url = sym_cfg.ws_url.clone();

        let bar_config = match sym_cfg.bar_type.as_str() {
            "time_1m" => BarConfig::TimeBased {
                interval_secs: sym_cfg.bar_param as u64,
            },
            "volume_100k" => BarConfig::VolumeBased {
                threshold: sym_cfg.bar_param,
            },
            other => {
                error!(bar_type = %other, "Unknown bar type, skipping symbol");
                continue;
            }
        };

        let ingester = MarketIngester::new(
            symbol.clone(),
            ws_url,
            bar_config,
            market_tx.clone(),
        );

        let handle = tokio::spawn(async move {
            if let Err(e) = ingester.run().await {
                error!(symbol = %symbol, error = %e, "Ingester failed");
            }
        });
        ingest_handles.push(handle);
    }

    drop(market_tx);

    let risk_config = ExecRiskConfig {
        max_position_size: config.risk.max_position_size,
        max_daily_loss: config.risk.max_daily_loss,
        max_spread_bps: config.risk.max_spread_bps,
    };
    let _exec_engine = ExecutionEngine::new(risk_config, exec_tx.clone());

    let exchange = BinanceFutures::new(
        config.exchange.api_key.clone(),
        config.exchange.api_secret.clone(),
        config.exchange.testnet,
    );
    let reconciler = Reconciler::new(exchange, 60, exec_tx.clone());

    let recon_handle = tokio::spawn(async move {
        reconciler.run().await;
    });

    let shutdown = async {
        signal::ctrl_c().await.expect("Failed to install Ctrl+C handler");
        info!("Shutdown signal received");
    };

    tokio::select! {
        _ = shutdown => {
            info!("Initiating graceful shutdown...");
        }
        _ = async {
            while let Some(event) = market_rx.recv().await {
                tracing::debug!(event_id = %event.event_id, "Received market event");
            }
        } => {
            info!("Market channel closed");
        }
    }

    for handle in ingest_handles {
        handle.abort();
    }
    recon_handle.abort();

    info!("Nemesis stopped gracefully");
    Ok(())
}
