use std::fs::File;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::sync::Arc;

use prost::Message;
use serde::Deserialize;

use nemesis_core::MarketTick;
use nemesis_core::NoopMetrics;
use nemesis_market::{BarBuilder, BarConfig};

#[derive(Deserialize)]
struct BacktestConfig {
    symbol: String,
    bar_config: BarConfigParams,
    input_file: String,
}

#[derive(Deserialize)]
#[serde(tag = "type")]
enum BarConfigParams {
    #[serde(rename = "time_1m")]
    Time1m { interval_secs: u64 },
    #[serde(rename = "volume_100k")]
    Volume100k { threshold: f64 },
}

fn main() -> anyhow::Result<()> {
    let mut config_str = String::new();
    io::stdin().lock().read_line(&mut config_str)?;
    let config: BacktestConfig = serde_json::from_str(&config_str)?;

    let bar_config = match config.bar_config {
        BarConfigParams::Time1m { interval_secs } => BarConfig::TimeBased { interval_secs },
        BarConfigParams::Volume100k { threshold } => BarConfig::VolumeBased { threshold },
    };

    let metrics = Arc::new(NoopMetrics);
    let mut builder =
        BarBuilder::new(config.symbol, "backtest".into(), bar_config).with_metrics(metrics);

    let file = File::open(&config.input_file)?;
    let reader = BufReader::new(file);
    let stdout = io::stdout();
    let mut out = BufWriter::new(stdout.lock());

    let mut tick_count: u64 = 0;
    let mut bar_count: u64 = 0;

    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() != 4 {
            continue;
        }

        let tick = MarketTick {
            price: parts[1].parse()?,
            quantity: parts[2].parse()?,
            is_buyer_maker: parts[3].parse::<bool>()?,
        };
        let ts_us: i64 = parts[0].parse()?;
        tick_count += 1;

        if let Some(envelope) = builder.on_tick(&tick, tick_count, ts_us) {
            let mut buf = Vec::with_capacity(envelope.encoded_len());
            envelope.encode(&mut buf)?;

            let len = (buf.len() as u32).to_be_bytes();
            out.write_all(&len)?;
            out.write_all(&buf)?;
            bar_count += 1;
        }
    }

    if let Some(envelope) = builder.force_close("end_of_file") {
        let mut buf = Vec::with_capacity(envelope.encoded_len());
        envelope.encode(&mut buf)?;
        let len = (buf.len() as u32).to_be_bytes();
        out.write_all(&len)?;
        out.write_all(&buf)?;
        bar_count += 1;
    }

    out.flush()?;
    eprintln!(
        "Replay complete: {} ticks -> {} bars",
        tick_count, bar_count
    );
    Ok(())
}
