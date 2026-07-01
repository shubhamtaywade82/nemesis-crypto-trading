#!/usr/bin/env python3
"""CLI entry point for deterministic backtests."""
import argparse
from pathlib import Path

from nemesis_alpha.backtest import BacktestHarness
from nemesis_alpha.strategy import ExampleMomentumStrategy


def main():
    parser = argparse.ArgumentParser(description="Run Nemesis backtest")
    parser.add_argument("--tick-file", required=True, type=Path)
    parser.add_argument("--symbol", default="BTCUSDT-PERP")
    parser.add_argument("--rust-binary", required=True, type=Path)
    parser.add_argument("--bar-type", choices=["time_1m", "volume_100k"], default="volume_100k")
    args = parser.parse_args()

    bar_config: dict = {"type": args.bar_type}
    if args.bar_type == "volume_100k":
        bar_config["threshold"] = 100_000.0
    else:
        bar_config["interval_secs"] = 60

    harness = BacktestHarness(
        rust_binary_path=str(args.rust_binary),
        symbol=args.symbol,
        bar_config=bar_config,
    )

    result = harness.run(args.tick_file, ExampleMomentumStrategy)

    print(f"\n{'='*50}")
    print(f"Backtest Results: {args.symbol}")
    print(f"{'='*50}")
    print(f"Bars Processed:   {result.bars_processed}")
    print(f"Corrupted Bars:   {result.corrupted_bars}")
    print(f"Total Trades:     {result.total_trades}")
    print(f"Total Return:     {result.total_return:.4%}")
    print(f"Sharpe Ratio:     {result.sharpe_ratio:.3f}")
    print(f"Max Drawdown:     {result.max_drawdown:.4%}")
    print(f"Win Rate:         {result.win_rate:.4%}")
    print(f"Avg Slippage:     {result.avg_slippage_bps:.1f} bps")
    print(f"{'='*50}\n")


if __name__ == "__main__":
    main()
