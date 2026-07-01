from __future__ import annotations
import logging

from nemesis_alpha.strategy import BaseStrategy
from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.signals import SignalEmitter, SignalSide, SignalType

logger = logging.getLogger(__name__)


class LiquiditySweepStrategy(BaseStrategy):
    """Exploit stop-loss cascades / liquidity sweeps on altcoin L1.

    Logic:
    - Track recent range (highest high, lowest low) over lookback bars.
    - A **bullish sweep** occurs when price breaks below the recent low with
      heavy sell aggression (stops being hunted), then closes back inside the
      range — indicating the selling was exhausted by triggered stops.
    - A **bearish sweep** occurs symmetrically above the recent high with heavy
      buy aggression.
    - Enter counter-trend at bar close; exit via TP/SL based on ATR.

    The backtest engine only supports long positions, so bearish sweeps serve as
    early-exit signals for existing longs rather than short entries.

    Parameters (walk-forward optimizable):
    - lookback: Bars for range & ATR calculation.
    - sweep_pct: Min % price must extend beyond range to qualify as a sweep.
    - vol_ratio: Min ratio of dominant-side / opposite-side volume (e.g., 1.5
      means sell volume must be 1.5x buy volume for a bullish sweep).
    - tp_atr: Take-profit in ATR multiples.
    - sl_atr: Stop-loss in ATR multiples.
    - quantity: Position size in base currency units (0.001 ETH).
    """

    def __init__(
        self,
        symbol: str,
        lookback: int = 20,
        sweep_pct: float = 0.001,
        vol_ratio: float = 1.5,
        tp_atr: float = 1.5,
        sl_atr: float = 0.5,
        quantity: float = 0.001,
    ):
        super().__init__(symbol)
        self.lookback = lookback
        self.sweep_pct = sweep_pct
        self.vol_ratio = vol_ratio
        self.tp_atr = tp_atr
        self.sl_atr = sl_atr
        self.quantity = quantity
        self._bars: list[ConsumedBar] = []
        self._in_position = False
        self._entry_price = 0.0
        self._stop_loss = 0.0
        self._take_profit = 0.0
        self._bar_count = 0

    async def on_bar(self, bar: ConsumedBar) -> bytes | None:
        if bar.is_corrupted:
            return None

        self._bars.append(bar)
        self._bar_count += 1

        if len(self._bars) > 500:
            self._bars = self._bars[-500:]

        if len(self._bars) < self.lookback + 2:
            return None

        recent = self._bars[-(self.lookback + 1):-1]
        recent_high = max(b.high for b in recent)
        recent_low = min(b.low for b in recent)

        atr = self._compute_atr(recent)
        if atr < 1e-8:
            return None

        if self._in_position:
            exit_signal = self._check_exit(bar, recent_high, recent_low, atr)
            if exit_signal:
                self._in_position = False
                return exit_signal

        if not self._in_position:
            entry = self._check_entry(bar, recent_high, recent_low, atr)
            if entry is not None:
                self._in_position = True
                self._entry_price = bar.close
                self._stop_loss = bar.close - atr * self.sl_atr
                self._take_profit = bar.close + atr * self.tp_atr
                return entry

        return None

    def _compute_atr(self, bars: list[ConsumedBar]) -> float:
        n = min(10, len(bars))
        return sum(b.high - b.low for b in bars[-n:]) / n

    def _check_exit(
        self,
        bar: ConsumedBar,
        recent_high: float,
        recent_low: float,
        atr: float,
    ) -> bytes | None:
        if bar.low <= self._stop_loss:
            return self._sell_signal(
                bar, 1.0, f"SL hit at {bar.close:.2f} (stop={self._stop_loss:.2f})",
            )
        if bar.high >= self._take_profit:
            return self._sell_signal(
                bar, 1.0, f"TP hit at {bar.close:.2f} (target={self._take_profit:.2f})",
            )
        bearish_exit = self._check_bearish_exit(bar, recent_high)
        if bearish_exit:
            return bearish_exit
        return None

    def _check_bearish_exit(self, bar: ConsumedBar, recent_high: float) -> bytes | None:
        if bar.high > recent_high * (1 + self.sweep_pct) and bar.close < recent_high:
            buy_vol_ratio = bar.buy_volume / max(bar.sell_volume, 1e-8)
            if buy_vol_ratio >= self.vol_ratio:
                return self._sell_signal(
                    bar,
                    min(buy_vol_ratio / (self.vol_ratio * 2), 1.0),
                    f"Bearish sweep exit: high={bar.high:.2f} > {recent_high:.2f}, buy/sell={buy_vol_ratio:.2f}",
                )
        return None

    def _check_entry(
        self,
        bar: ConsumedBar,
        recent_high: float,
        recent_low: float,
        atr: float,
    ) -> bytes | None:
        if bar.low < recent_low * (1 - self.sweep_pct) and bar.close > recent_low:
            sell_vol_ratio = bar.sell_volume / max(bar.buy_volume, 1e-8)
            if sell_vol_ratio >= self.vol_ratio:
                confidence = min(sell_vol_ratio / (self.vol_ratio * 2), 1.0)
                return self._buy_signal(
                    bar,
                    confidence,
                    f"Bullish sweep: low={bar.low:.2f} < {recent_low:.2f}, sell/buy={sell_vol_ratio:.2f}",
                )
        return None

    def _buy_signal(self, bar: ConsumedBar, confidence: float, rationale: str) -> bytes:
        return self.emitter.build_signal(
            symbol=self.symbol,
            side=SignalSide.BUY,
            signal_type=SignalType.MARKET,
            price=None,
            quantity=self.quantity,
            confidence=confidence,
            rationale=rationale,
        )

    def _sell_signal(self, bar: ConsumedBar, confidence: float, rationale: str) -> bytes:
        return self.emitter.build_signal(
            symbol=self.symbol,
            side=SignalSide.SELL,
            signal_type=SignalType.MARKET,
            price=None,
            quantity=self.quantity,
            confidence=confidence,
            rationale=rationale,
        )
