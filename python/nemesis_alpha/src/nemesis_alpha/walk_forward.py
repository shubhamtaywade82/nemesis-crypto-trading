from __future__ import annotations
from pathlib import Path
from typing import Type

from nemesis_alpha.backtest_engine import BacktestEngine, BacktestResult
from nemesis_alpha.analytics import compute_metrics, PerformanceMetrics
from nemesis_alpha.strategy import BaseStrategy


class WalkForwardValidator:
    def __init__(self, rust_binary: str, tick_file: Path):
        self.engine = BacktestEngine(rust_binary)
        self.tick_file = tick_file

    async def validate(
        self,
        symbol: str,
        bar_config: dict,
        strategy_cls: Type[BaseStrategy],
        in_sample_pct: float = 0.7,
        **strategy_kwargs,
    ) -> dict[str, PerformanceMetrics]:
        result = await self.engine.run(
            self.tick_file, symbol, bar_config, strategy_cls, **strategy_kwargs,
        )

        split_idx = int(len(result.equity_curve) * max(min(in_sample_pct, 1.0), 0.0))
        is_equity = result.equity_curve[:split_idx]
        oos_equity = result.equity_curve[split_idx:]

        is_return = (is_equity[-1] - is_equity[0]) / is_equity[0] if len(is_equity) > 1 else 0.0
        oos_return = (oos_equity[-1] - oos_equity[0]) / oos_equity[0] if len(oos_equity) > 1 else 0.0

        is_metrics = compute_metrics(
            BacktestResult(
                trades=result.trades,
                bars_processed=result.bars_processed,
                corrupted_bars_skipped=result.corrupted_bars_skipped,
                signals_generated=result.signals_generated,
                equity_curve=is_equity,
            )
        )
        oos_metrics = compute_metrics(
            BacktestResult(
                trades=result.trades,
                bars_processed=result.bars_processed,
                corrupted_bars_skipped=result.corrupted_bars_skipped,
                signals_generated=result.signals_generated,
                equity_curve=oos_equity,
            )
        )

        return {"in_sample": is_metrics, "out_of_sample": oos_metrics}
