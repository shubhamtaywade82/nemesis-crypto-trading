from __future__ import annotations

import math

import pytest

from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.strategy import ExampleMomentumStrategy
from nemesis_alpha.ofi_strategy import OFIMeanReversionStrategy
from nemesis_alpha.liquidity_sweep_strategy import LiquiditySweepStrategy


def make_bar(
    *,
    open_: float = 100.0,
    high: float = 100.0,
    low: float = 100.0,
    close: float = 100.0,
    volume: float = 0.0,
    buy_volume: float = 0.0,
    sell_volume: float = 0.0,
    is_corrupted: bool = False,
    first_seq: int = 1,
    last_seq: int = 1,
) -> ConsumedBar:
    return ConsumedBar(
        open=open_,
        high=high,
        low=low,
        close=close,
        volume=volume,
        buy_volume=buy_volume,
        sell_volume=sell_volume,
        first_seq=first_seq,
        last_seq=last_seq,
        is_corrupted=is_corrupted,
    )


@pytest.mark.asyncio
async def test_momentum_generates_buy_signal():
    strategy = ExampleMomentumStrategy(symbol="BTCUSDT", lookback=4, threshold=0.001)
    strategy._bar_history.clear()

    for idx in range(5):
        await strategy.on_bar(make_bar(close=100.0 + idx, first_seq=idx + 1, last_seq=idx + 1))

    momentum_bar = make_bar(close=105.5, first_seq=6, last_seq=6)
    signal = await strategy.on_bar(momentum_bar)
    assert signal is not None


@pytest.mark.asyncio
async def test_momentum_skips_corrupted_bars():
    strategy = ExampleMomentumStrategy(symbol="BTCUSDT", lookback=2, threshold=0.01)
    strategy._bar_history.clear()

    bar1 = make_bar(close=100.0, first_seq=1, last_seq=1)
    bar2 = make_bar(close=101.0, first_seq=2, last_seq=2)
    bar3 = make_bar(close=102.0, first_seq=3, last_seq=3, is_corrupted=True)

    await strategy.on_bar(bar1)
    await strategy.on_bar(bar2)
    assert len(strategy._bar_history) == 2

    signal = await strategy.on_bar(bar3)
    assert signal is None
    assert len(strategy._bar_history) == 3


@pytest.mark.asyncio
async def test_ofi_mean_reversion_generates_sell_on_buyer_exhaustion():
    strategy = OFIMeanReversionStrategy(
        symbol="BTCUSDT",
        lookback=6,
        z_threshold=2.0,
        momentum_divergence=0.0005,
        quantity=0.001,
    )
    strategy._bar_count = 5
    strategy._max_z = 1.0
    strategy._max_mom = 0.0001
    strategy.ofi_history = [100.0, -80.0, 120.0, -90.0, 110.0]
    strategy.price_history = [100.0, 99.9, 100.1, 99.8, 100.2]

    signal = await strategy.on_bar(
        make_bar(
            close=99.5,
            buy_volume=1_000_000.0,
            sell_volume=0.0,
            first_seq=6,
            last_seq=6,
        )
    )
    assert signal is None
    assert strategy._max_z >= 1.0


@pytest.mark.asyncio
async def test_liquidity_sweep_bullish_entry_and_exit():
    strategy = LiquiditySweepStrategy(
        symbol="ETHUSDT",
        lookback=5,
        sweep_pct=0.0,
        vol_ratio=1.0,
        tp_atr=1.5,
        sl_atr=0.5,
        quantity=0.001,
    )
    strategy._in_position = False
    strategy._bars.clear()

    recent_prices = [100.0, 100.1, 99.9, 100.2, 99.8, 100.0]
    for idx, price in enumerate(recent_prices):
        await strategy.on_bar(
            make_bar(
                close=price,
                low=price - 0.1,
                high=price + 0.1,
                buy_volume=100.0,
                sell_volume=100.0,
                first_seq=idx + 1,
                last_seq=idx + 1,
            )
        )
    assert len(strategy._bars) == 6

    sweep_bar = make_bar(
        close=102.0,
        low=min(recent_prices) - 0.5,
        high=min(recent_prices) + 4.5,
        buy_volume=100.0,
        sell_volume=100.0,
        first_seq=7,
        last_seq=7,
    )
    await strategy.on_bar(sweep_bar)

    recent = strategy._bars[-(strategy.lookback + 1):-1]
    recent_low = min(b.low for b in recent)
    atr = strategy._compute_atr(recent)
    assert math.isclose(strategy._entry_price, sweep_bar.close)
    assert math.isclose(strategy._stop_loss, sweep_bar.close - atr * strategy.sl_atr)
    assert math.isclose(strategy._take_profit, sweep_bar.close + atr * strategy.tp_atr)


@pytest.mark.asyncio
async def test_liquidity_sweep_avoids_flat_atr():
    strategy = LiquiditySweepStrategy(symbol="ETHUSDT", lookback=10, tp_atr=1.0, sl_atr=1.0)
    strategy._bars = [make_bar(high=100.0, low=100.0) for _ in range(10)]

    bar = make_bar(high=100.0, low=100.0, first_seq=11, last_seq=11)
    signal = await strategy.on_bar(bar)
    assert signal is None


def test_analytics_metrics_coverage():
    from nemesis_alpha.analytics import PerformanceMetrics, compute_metrics
    from nemesis_alpha.backtest_engine import BacktestResult

    result = BacktestResult(
        trades=[
            type("TradeRecord", (), {})() for _ in range(3)
        ],
        bars_processed=10,
        corrupted_bars_skipped=2,
        signals_generated=4,
        equity_curve=[10000.0 + i * 500 for i in range(11)] + [10500.0],
    )
    for idx, trade in enumerate(result.trades):
        trade.is_open = False
        trade.pnl = 30.0 if idx < 2 else -20.0
        trade.entry_time = 0
        trade.exit_time = 1
        trade.side = "LONG"
        trade.entry_price = 100.0
        trade.exit_price = 101.0 if idx < 2 else 99.0
        trade.quantity = 1.0

    metrics = compute_metrics(result)

    assert math.isclose(metrics.total_return, (10500.0 - 10000.0) / 10000.0, rel_tol=1e-9)
    assert math.isclose(metrics.total_return, 0.05, rel_tol=1e-12)

    synthetic_result = BacktestResult(
        trades=[],
        bars_processed=0,
        corrupted_bars_skipped=0,
        signals_generated=0,
        equity_curve=[100.0, 105.0],
    )
    synthetic_metrics = compute_metrics(synthetic_result)
    assert math.isclose(synthetic_metrics.annualized_return, 0.05)
