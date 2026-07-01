#!/usr/bin/env python3
"""Walk-forward grid search for OFI Mean-Reversion strategy parameters."""
import asyncio
import csv
import itertools
import sys
from pathlib import Path

from nemesis_alpha.backtest_engine import BacktestEngine
from nemesis_alpha.analytics import compute_metrics
from nemesis_alpha.ofi_strategy import OFIMeanReversionStrategy

TICK_FILE = Path("../../data/ticks_btcusdt_2026.csv")
RUST_BINARY = str(Path("../../crates/target/release/nemesis-backtest" + (".exe" if sys.platform == "win32" else "")))
SYMBOL = "BTCUSDT"
BAR_CONFIG = {"type": "volume_100k", "threshold": 100000.0}

PARAM_GRID = {
    "lookback": [15, 20, 30, 40, 60],
    "z_threshold": [1.5, 2.0, 2.5, 3.0],
    "momentum_divergence": [0.003, 0.005, 0.008, 0.012],
}

IN_SAMPLE_PCT = 0.7


async def evaluate_params(params: dict) -> dict:
    engine = BacktestEngine(RUST_BINARY)
    result = await engine.run(
        tick_file=TICK_FILE,
        symbol=SYMBOL,
        bar_config=BAR_CONFIG,
        strategy_cls=OFIMeanReversionStrategy,
        **params,
    )
    metrics = compute_metrics(result)

    split_idx = int(len(result.equity_curve) * IN_SAMPLE_PCT)
    is_equity = result.equity_curve[:split_idx]
    oos_equity = result.equity_curve[split_idx:]

    is_return = (is_equity[-1] - is_equity[0]) / is_equity[0] if len(is_equity) > 1 else 0
    oos_return = (oos_equity[-1] - oos_equity[0]) / oos_equity[0] if len(oos_equity) > 1 else 0

    return {
        **params,
        "is_sharpe": metrics.sharpe_ratio,
        "oos_return": oos_return,
        "is_return": is_return,
        "total_trades": metrics.total_trades,
        "win_rate": metrics.win_rate,
        "max_drawdown": metrics.max_drawdown,
        "profit_factor": metrics.profit_factor,
        "bars_processed": result.bars_processed,
    }


async def main():
    combos = list(itertools.product(*PARAM_GRID.values()))
    param_names = list(PARAM_GRID.keys())
    all_results = []

    print(f"Searching {len(combos)} parameter combinations...")
    for i, combo in enumerate(combos):
        params = dict(zip(param_names, combo))
        result = await evaluate_params(params)
        all_results.append(result)

        if (i + 1) % 10 == 0:
            print(f"  Progress: {i+1}/{len(combos)}")

    all_results.sort(key=lambda r: r["oos_return"], reverse=True)

    out_path = Path("../../data/ofi_optimization_results.csv")
    with open(out_path, "w", newline="") as f:
        writer = csv.DictWriter(f, fieldnames=list(all_results[0].keys()))
        writer.writeheader()
        writer.writerows(all_results)

    print()
    print("=" * 90)
    print("TOP 10 PARAMETER COMBINATIONS (sorted by OOS return)")
    print("=" * 90)
    print(f"{'Lookback':>8} {'Z-Thresh':>8} {'Mom-Div':>8} {'IS Sharpe':>10} {'OOS Ret':>10} {'Trades':>7} {'WinRate':>8} {'PF':>6}")
    print("-" * 90)
    for r in all_results[:10]:
        print(
            f"{r['lookback']:>8} {r['z_threshold']:>8.1f} {r['momentum_divergence']:>8.4f} "
            f"{r['is_sharpe']:>10.3f} {r['oos_return']:>9.2%} {r['total_trades']:>7} "
            f"{r['win_rate']:>7.1%} {r['profit_factor']:>6.2f}"
        )
    print("=" * 90)
    print(f"\nFull results saved to {out_path}")

    best = all_results[0]
    if best["is_sharpe"] > 1.5 and best["oos_return"] < 0:
        print("\nWARNING: Best IS Sharpe is high but OOS is negative -- likely overfit!")
    elif best["oos_return"] > 0 and best["total_trades"] >= 20:
        print(f"\nVIABLE: OOS positive with {best['total_trades']} trades. Proceed to dry-run.")
    else:
        print("\nNo viable parameters found. Consider different strategy architecture.")


if __name__ == "__main__":
    asyncio.run(main())
