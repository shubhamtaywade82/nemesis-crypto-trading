#!/usr/bin/env python3
"""Unified Nemesis backtest CLI: download -> replay -> analyze -> report."""
import argparse
import asyncio
import json
from pathlib import Path
from nemesis_alpha.backtest_engine import BacktestEngine
from nemesis_alpha.analytics import compute_metrics
from nemesis_alpha.strategy import ExampleMomentumStrategy


async def run_backtest(
    tick_file: Path,
    symbol: str,
    bar_type: str,
    bar_param: float,
    rust_binary: str,
) -> None:
    bar_config: dict = {"type": bar_type}
    if bar_type == "volume_100k":
        bar_config["threshold"] = bar_param
    else:
        bar_config["interval_secs"] = int(bar_param)

    engine = BacktestEngine(rust_binary)
    result = await engine.run(
        tick_file=tick_file,
        symbol=symbol,
        bar_config=bar_config,
        strategy_cls=ExampleMomentumStrategy,
    )

    metrics = compute_metrics(result)

    print("\n" + "=" * 60)
    print(f"  NEMESIS BACKTEST RESULTS: {symbol}")
    print("=" * 60)
    print(f"  Bars Processed:     {result.bars_processed:>12,}")
    print(f"  Corrupted Skipped:  {result.corrupted_bars_skipped:>12,}")
    print(f"  Signals Generated:  {result.signals_generated:>12,}")
    print(f"  Total Trades:       {metrics.total_trades:>12,}")
    print(f"  Win Rate:           {metrics.win_rate:>11.1%}")
    print(f"  Profit Factor:      {metrics.profit_factor:>12.3f}")
    print(f"  Sharpe Ratio:       {metrics.sharpe_ratio:>12.3f}")
    print(f"  Sortino Ratio:      {metrics.sortino_ratio:>12.3f}")
    print(f"  Max Drawdown:       {metrics.max_drawdown:>11.2%}")
    print(f"  Max DD Duration:    {metrics.max_drawdown_duration_bars:>9,} bars")
    print(f"  Total Return:       {metrics.total_return:>11.2%}")
    print(f"  Annualized Return:  {metrics.annualized_return:>11.2%}")
    print("=" * 60 + "\n")

    out_path = tick_file.with_suffix(".results.json")
    out_path.write_text(json.dumps({
        "symbol": symbol,
        "bar_config": bar_config,
        "metrics": {
            "sharpe": metrics.sharpe_ratio,
            "sortino": metrics.sortino_ratio,
            "max_drawdown": metrics.max_drawdown,
            "win_rate": metrics.win_rate,
            "profit_factor": metrics.profit_factor,
            "total_return": metrics.total_return,
            "total_trades": metrics.total_trades,
        },
        "bars_processed": result.bars_processed,
        "corrupted_bars": result.corrupted_bars_skipped,
    }, indent=2))
    print(f"Results saved to {out_path}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Run Nemesis backtest")
    parser.add_argument("--tick-file", required=True, type=Path)
    parser.add_argument("--symbol", default="BTCUSDT")
    parser.add_argument("--bar-type", choices=["time_1m", "volume_100k"], default="volume_100k")
    parser.add_argument("--bar-param", type=float, default=100000.0,
                        help="Volume threshold (USDT) or time interval (seconds)")
    parser.add_argument("--rust-binary", default="crates/target/release/nemesis-backtest")
    args = parser.parse_args()

    if not args.tick_file.exists():
        print(f"Tick file not found: {args.tick_file}")
        print("   Run download_ticks.py first:")
        print("   python scripts/download_ticks.py --start 2026-04-01T00:00:00 --end 2026-07-01T00:00:00")
        exit(1)

    asyncio.run(run_backtest(
        tick_file=args.tick_file,
        symbol=args.symbol,
        bar_type=args.bar_type,
        bar_param=args.bar_param,
        rust_binary=args.rust_binary,
    ))
