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
        _ = in_sample_pct
        result = await self.engine.run(
            self.tick_file, symbol, bar_config, strategy_cls, **strategy_kwargs,
        )
        metrics = compute_metrics(result)
        return {"full_sample": metrics}
