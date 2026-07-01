# Nemesis

Hybrid Rust/Python algorithmic cryptocurrency trading system. Connects to Binance Futures via WebSocket, builds OHLCV bars from individual trades, runs trading strategies, and executes orders through paper or live exchange adapters.

## Features

- **Real-time market data ingestion** — Binance Futures WebSocket (`aggTrade` stream) with automatic reconnection and feed health monitoring
- **Deterministic bar building** — Volume-based and time-based OHLCV bar construction with sequence gap detection and corruption marking
- **Paper exchange** — In-memory order book with price-time priority matching for simulated execution
- **Live exchange** — HMAC-SHA256 signed REST client for Binance Futures API
- **Risk management** — Configurable max position size, daily loss limit, spread checks, and kill switch
- **Observability** — Prometheus metrics, structured JSON logging (Loki-compatible), Grafana dashboard
- **Python alpha research** — Backtest harness consuming protobuf bars from the Rust binary, strategy development framework, walk-forward validation, and performance analytics
- **TimescaleDB persistence** — Bar data stored in hypertables, orders/positions/audit log in PostgreSQL
- **Protobuf event contract** — Universal `EventEnvelope` for all system events, shared between Rust and Python

## Architecture

```
Binance Futures
   |
   v
MarketIngester (nemesis-market)
   |-- parses aggTrade JSON -> MarketTick protobuf
   |-- feeds through BarBuilder (volume/time based)
   |-- publishes EventEnvelopes via EventPublisher
   |-- SessionMonitor tracks feed health
   v
EventEnvelope (protobuf) ----> Python Consumer (nemesis-alpha)
   |                                  |
   v                                  v
ExecutionEngine (nemesis-execution)   BacktestEngine
   |-- RiskEngine validates orders     |-- Replay ticks -> bars
   |-- PaperExchange (simulated)       |-- Strategy -> signals
   |-- BinanceFutures (live)           |-- Compute metrics
   |-- Reconciler (health checks)
   v
PersistenceWriter (nemesis-execution)
   |-- Writes bars  -> TimescaleDB hypertable
   |-- Writes audit_log -> PostgreSQL
   v
HTTP Server (nemesis-main)
   |-- /health endpoint
   |-- /metrics (Prometheus)
   v
Grafana / Loki / Prometheus
```

## Project Structure

```
├── crates/
│   ├── nemesis-core/          # Shared types, protobuf codegen, MetricsRecorder trait
│   ├── nemesis-market/        # Market data ingestion, bar building, backtest binary
│   ├── nemesis-execution/     # Exchange adapters, execution engine, risk, persistence
│   └── nemesis-main/          # Main binary: wiring, config, HTTP server, metrics
├── python/nemesis_alpha/      # Python alpha research package
│   ├── src/nemesis_alpha/
│   │   ├── strategy.py        # BaseStrategy + ExampleMomentumStrategy
│   │   ├── backtest.py        # Sync backtest harness (subprocesses Rust binary)
│   │   ├── backtest_engine.py # Async backtest engine with trade simulation
│   │   ├── analytics.py       # Sharpe, Sortino, max drawdown, etc.
│   │   ├── walk_forward.py    # Walk-forward validation
│   │   ├── signals.py         # TradeSignal protobuf builder
│   │   ├── consumer.py        # EventEnvelope deserializer
│   │   └── db.py              # Read-only asyncpg DB interface
│   └── scripts/
│       ├── download_ticks.py  # Binance historical tick downloader
│       ├── run_backtest.py    # Unified backtest CLI
│       └── generate_proto.py  # Python protobuf stub generator
├── proto/
│   └── envelope.proto         # Universal event envelope (protobuf v3)
├── migrations/                # TimescaleDB schema migrations
├── config/
│   └── nemesis.toml           # Main application configuration
├── infra/
│   ├── docker-compose.yml     # TimescaleDB + Redpanda for local dev
│   ├── Dockerfile             # Multi-stage container build
│   ├── k8s/                   # Kubernetes Deployment + Service manifests
│   ├── grafana/               # Grafana dashboard definition
│   ├── loki/                  # Promtail log scraping config
│   └── backup/                # K8s CronJob for DB backups
├── docs/
│   └── RUNBOOK.md             # Incident response procedures
└── data/
    └── ticks_btcusdt_2026.csv # Historical tick data sample
```

## Prerequisites

- Rust 1.82+ with `wasm32-unknown-unknown` target (optional)
- Python 3.12+
- PostgreSQL 16 with TimescaleDB extension
- Docker & Docker Compose (for local infrastructure)

## Getting Started

### 1. Start infrastructure

```bash
docker compose -f infra/docker-compose.yml up -d
```

This starts TimescaleDB on port 5432 and Redpanda (Kafka-compatible) on port 9092.

### 2. Run database migrations

```bash
cargo install sqlx-cli
DATABASE_URL=postgres://nemesis:nemesis_dev@localhost:5432/nemesis \
  sqlx migrate run --source migrations/
```

### 3. Build and run (paper trading)

```bash
cd crates
cargo run --release --bin nemesis
```

The application reads `config/nemesis.toml` by default. In `dry_run` mode, the system connects to Binance WebSocket for live market data but executes orders against the in-memory `PaperExchange`.

### 4. Run backtest

```bash
cd crates
cargo run --release --bin nemesis-backtest -- \
  --ticks ../data/ticks_btcusdt_2026.csv \
  --symbol BTCUSDT-PERP \
  --bar-type volume_100k \
  --bar-param 100000.0 \
  --timeout 60 > bars.bin
```

Outputs length-prefixed protobuf `EventEnvelope` bytes to stdout.

### 5. Python backtest

```bash
cd python/nemesis_alpha
pip install -e ".[dev]"
python scripts/download_ticks.py --symbol BTCUSDT --days 30
python scripts/run_backtest.py \
  --ticks ../../data/ticks_btcusdt_2026.csv \
  --symbol BTCUSDT-PERP \
  --bar-type volume_100k \
  --bar-param 100000.0 \
  --rust-binary ../../crates/target/release/nemesis-backtest
```

## Configuration

See `config/nemesis.toml` for the full config. Key settings:

| Key | Default | Description |
|-----|---------|-------------|
| `exchange.name` | `"binance"` | Exchange adapter |
| `exchange.dry_run` | `true` | Use paper exchange |
| `exchange.testnet` | `false` | Use Binance testnet API |
| `symbols[].bar_type` | — | `volume_<vol>` or `time_<secs>` |
| `risk.max_position_size` | `0.01` | BTC max position |
| `risk.max_daily_loss` | `50.0` | USD daily loss limit |
| `logging.format` | `"json"` | Log format (`json` or `pretty`) |

API keys can be resolved from AWS Secrets Manager by using `${SECRET_NAME}` syntax in the config file, or set via environment variables `BINANCE_API_KEY` / `BINANCE_API_SECRET`.

Set `NEMESIS_MAINNET_CONFIRM=YES` to run with `testnet=false` and `dry_run=false`. Confirm twice before deploying.

## Python Alpha Research

The `nemesis-alpha` package provides a strategy development framework:

```python
from nemesis_alpha.strategy import BaseStrategy

class MyStrategy(BaseStrategy):
    def on_bar(self, bar) -> TradeSignal | None:
        if bar.close > bar.open * 1.01:
            return self.signal_buy(bar.close, 0.01, confidence=0.6,
                                   rationale="bar up >1%")
        return None
```

```bash
python scripts/run_backtest.py --strategy my_module.MyStrategy ...
```

Run analytics:

```python
from nemesis_alpha.analytics import compute_metrics
from nemesis_alpha.backtest_engine import BacktestEngine

engine = BacktestEngine(...)
result = await engine.run()
metrics = compute_metrics(result.trades, result.equity_curve)
print(metrics.sharpe, metrics.max_drawdown)
```

## API Endpoints

| Endpoint | Description |
|----------|-------------|
| `GET /health` | Health check: status, version, uptime, database connectivity |
| `GET /metrics` | Prometheus text format metrics |

## Infrastructure

### Docker

```bash
docker compose -f infra/docker-compose.yml up -d
```

### Kubernetes

```bash
kubectl apply -f infra/k8s/
```

Exposes a ClusterIP `nemesis` service on port 9090. See `infra/k8s/` for resource limits, probes, and service account configuration.

### Monitoring

A Grafana dashboard is available at `infra/grafana/nemesis-overview.json`. Import it into your Grafana instance and point it to the Prometheus datasource scraping the `/metrics` endpoint.

## Running Tests

```bash
# Rust tests
cd crates
DATABASE_URL=postgres://nemesis:nemesis_test@localhost:5432/nemesis_test \
  cargo test --all -- --test-threads=1

# Python tests
cd python/nemesis_alpha
pip install -e ".[dev]"
python scripts/generate_proto.py
pytest tests/ -v
```

## Commit Conventions

This repository uses [conventional commits](https://www.conventionalcommits.org/):

```
feat(core): add volume-weighted bar builder
fix(execution): handle exchange rate limit retry
docs(infra): update deployment guide
```

Valid scopes: `core`, `market`, `execution`, `main`, `python`, `proto`, `infra`, `ci`.

## Runbook

See [docs/RUNBOOK.md](docs/RUNBOOK.md) for operational procedures covering kill switch activation, stale feed recovery, reconciliation drift, emergency shutdown, and secret rotation.

## License

MIT
