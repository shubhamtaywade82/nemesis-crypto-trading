from __future__ import annotations
import math
from dataclasses import dataclass

from nemesis_alpha.backtest_engine import BacktestResult


@dataclass
class PerformanceMetrics:
    total_return: float
    annualized_return: float
    sharpe_ratio: float
    sortino_ratio: float
    max_drawdown: float
    max_drawdown_duration_bars: int
    win_rate: float
    profit_factor: float
    total_trades: int
    avg_trade_pnl: float
    bars_processed: int
    corrupted_bars: int


def compute_metrics(result: BacktestResult, risk_free_rate: float = 0.0) -> PerformanceMetrics:
    equity = result.equity_curve
    if len(equity) < 2:
        raise ValueError("Insufficient data for metrics")

    returns = [(equity[i] - equity[i - 1]) / equity[i - 1] for i in range(1, len(equity))]
    total_return = (equity[-1] - equity[0]) / equity[0]

    n_periods = len(returns)
    periods_per_year = 525_600
    annualized_return = (1 + total_return) ** (periods_per_year / max(n_periods, 1)) - 1

    mean_ret = sum(returns) / len(returns)
    variance = sum((r - mean_ret) ** 2 for r in returns) / max(len(returns) - 1, 1)
    std_ret = math.sqrt(variance)
    sharpe = (
        ((mean_ret - risk_free_rate / periods_per_year) / std_ret) * math.sqrt(periods_per_year)
        if std_ret > 0
        else 0.0
    )

    downside = [min(r, 0) ** 2 for r in returns]
    downside_var = sum(downside) / max(len(downside), 1)
    downside_std = math.sqrt(downside_var)
    sortino = (
        ((mean_ret - risk_free_rate / periods_per_year) / downside_std) * math.sqrt(periods_per_year)
        if downside_std > 0
        else 0.0
    )

    peak = equity[0]
    max_dd = 0.0
    current_dd_start = 0
    max_dd_duration = 0
    for i, val in enumerate(equity):
        if val > peak:
            peak = val
            current_dd_start = i
        dd = (peak - val) / peak
        if dd > max_dd:
            max_dd = dd
            max_dd_duration = i - current_dd_start

    closed_trades = [t for t in result.trades if not t.is_open]
    wins = [t for t in closed_trades if t.pnl > 0]
    losses = [t for t in closed_trades if t.pnl <= 0]
    gross_profit = sum(t.pnl for t in wins) if wins else 0.0
    gross_loss = abs(sum(t.pnl for t in losses)) if losses else 0.0

    return PerformanceMetrics(
        total_return=total_return,
        annualized_return=annualized_return,
        sharpe_ratio=sharpe,
        sortino_ratio=sortino,
        max_drawdown=max_dd,
        max_drawdown_duration_bars=max_dd_duration,
        win_rate=len(wins) / max(len(closed_trades), 1),
        profit_factor=gross_profit / gross_loss if gross_loss > 0 else float("inf"),
        total_trades=len(closed_trades),
        avg_trade_pnl=sum(t.pnl for t in closed_trades) / max(len(closed_trades), 1),
        bars_processed=result.bars_processed,
        corrupted_bars=result.corrupted_bars_skipped,
    )
