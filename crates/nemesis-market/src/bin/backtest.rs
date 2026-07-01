use std::io::{self, Read, Write};

use nemesis_core::EventEnvelope;
use nemesis_market::{BarBuilder, BarConfig};
use prost::Message;

fn main() -> anyhow::Result<()> {
    let mut config_str = String::new();
    io::stdin().read_to_string(&mut config_str)?;
    let config: serde_json::Value = serde_json::from_str(&config_str)?;

    let symbol = config["symbol"]
        .as_str()
        .unwrap_or("BTCUSDT-PERP")
        .to_string();
    let bar_type = config["bar_config"]["type"].as_str().unwrap_or("volume_100k");

    let bar_config = match bar_type {
        "time_1m" => BarConfig::TimeBased { interval_secs: 60 },
        "volume_100k" => {
            let threshold = config["bar_config"]["threshold"]
                .as_f64()
                .unwrap_or(100_000.0);
            BarConfig::VolumeBased { threshold }
        }
        _ => anyhow::bail!("Unknown bar type: {}", bar_type),
    };

    let mut builder = BarBuilder::new(symbol, "backtest".into(), bar_config);

    let input_file = config["input_file"].as_str();
    if let Some(path) = input_file {
        let content = std::fs::read_to_string(path)?;
        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }
            let tick: serde_json::Value = serde_json::from_str(line)?;
            let price = tick["price"].as_f64().unwrap_or(0.0);
            let quantity = tick["quantity"].as_f64().unwrap_or(0.0);
            let is_buyer_maker = tick["is_buyer_maker"].as_bool().unwrap_or(false);
            let seq = tick["seq"].as_u64().unwrap_or(0);
            let ts_us = tick["ts_us"].as_i64().unwrap_or(0);

            let market_tick = nemesis_core::MarketTick {
                price,
                quantity,
                is_buyer_maker,
            };

            if let Some(envelope) = builder.on_tick(&market_tick, seq, ts_us) {
                write_envelope(&envelope)?;
            }
        }
    } else {
        let mut buf = Vec::new();
        io::stdin().read_to_end(&mut buf)?;
    }

    if let Some(envelope) = builder.force_close("end-of-input") {
        write_envelope(&envelope)?;
    }

    Ok(())
}

fn write_envelope(envelope: &EventEnvelope) -> anyhow::Result<()> {
    let mut buf = Vec::new();
    envelope.encode(&mut buf)?;
    let len = (buf.len() as u32).to_be_bytes();
    let stdout = io::stdout();
    let mut handle = stdout.lock();
    handle.write_all(&len)?;
    handle.write_all(&buf)?;
    handle.flush()?;
    Ok(())
}
