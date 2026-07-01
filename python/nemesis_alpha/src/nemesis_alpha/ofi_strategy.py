from __future__ import annotations
import logging

from nemesis_alpha.strategy import BaseStrategy
from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.signals import SignalEmitter, SignalSide, SignalType

logger = logging.getLogger(__name__)


class OFIMeanReversionStrategy(BaseStrategy):
    """Mean-reversion strategy based on Order Flow Imbalance exhaustion.

    Logic:
    - Compute OFI = buy_volume - sell_volume over a rolling window
    - When OFI reaches extreme z-score AND price momentum diverges,
      it signals exhaustion of the current move
    - Enter counter-trend with tight stop at the bar's extreme

    Parameters (to be optimized via walk-forward):
    - lookback: Window size for OFI z-score calculation
    - z_threshold: Z-score threshold for "exhaustion" signal
    - momentum_divergence: Minimum price change % to confirm divergence
    """

    def __init__(
        self,
        symbol: str,
        lookback: int = 30,
        z_threshold: float = 2.0,
        momentum_divergence: float = 0.0005,
        quantity: float = 0.001,
    ):
        super().__init__(symbol)
        self.lookback = lookback
        self.z_threshold = z_threshold
        self.momentum_divergence = momentum_divergence
        self.quantity = quantity
        self.ofi_history: list[float] = []
        self.price_history: list[float] = []
        self._bar_count = 0
        self._max_z = 0.0
        self._max_mom = 0.0

    async def on_bar(self, bar: ConsumedBar) -> bytes | None:
        if bar.is_corrupted:
            return None

        ofi = bar.buy_volume - bar.sell_volume
        self.ofi_history.append(ofi)
        self.price_history.append(bar.close)

        max_len = self.lookback + 10
        if len(self.ofi_history) > max_len:
            self.ofi_history = self.ofi_history[-max_len:]
            self.price_history = self.price_history[-max_len:]

        if len(self.ofi_history) < self.lookback:
            return None

        window = self.ofi_history[-self.lookback:]
        mean_ofi = sum(window) / len(window)
        variance = sum((x - mean_ofi) ** 2 for x in window) / len(window)
        std_ofi = variance ** 0.5
        if std_ofi < 1e-8:
            return None
        current_z = (ofi - mean_ofi) / std_ofi

        if len(self.price_history) < self.lookback + 1:
            return None
        recent_momentum = (bar.close - self.price_history[-self.lookback]) / self.price_history[-self.lookback]

        self._bar_count += 1
        self._max_z = max(self._max_z, abs(current_z))
        self._max_mom = max(self._max_mom, abs(recent_momentum))

        if self._bar_count % 5000 == 0:
            logger.info(
                "OFI progress: bar=%d, |z|_max=%.2f, |mom|_max=%.6f, "
                "last_z=%.2f, last_mom=%.6f, ofi=%.0f, close=%.2f",
                self._bar_count, self._max_z, self._max_mom,
                current_z, recent_momentum, ofi, bar.close,
            )

        if self._bar_count == self.lookback * 2:
            window = self.ofi_history[-self.lookback:]
            logger.info(
                "OFI window stats: mean=%.0f, std=%.0f, "
                "min_ofi=%.0f, max_ofi=%.0f, "
                "pct_pos=%.1f%%",
                sum(window) / len(window), std_ofi,
                min(window), max(window),
                sum(1 for x in window if x > 0) / len(window) * 100,
            )

        if current_z > self.z_threshold and recent_momentum < -self.momentum_divergence:
            rationale = (
                f"OFI z={current_z:.2f} (buyer exhaustion), "
                f"momentum={recent_momentum:.4%} diverging"
            )
            return self.emitter.build_signal(
                symbol=self.symbol,
                side=SignalSide.SELL,
                signal_type=SignalType.LIMIT,
                price=bar.close,
                quantity=self.quantity,
                confidence=min(abs(current_z) / 4.0, 1.0),
                rationale=rationale,
            )

        if current_z < -self.z_threshold and recent_momentum > self.momentum_divergence:
            rationale = (
                f"OFI z={current_z:.2f} (seller exhaustion), "
                f"momentum={recent_momentum:.4%} diverging"
            )
            return self.emitter.build_signal(
                symbol=self.symbol,
                side=SignalSide.BUY,
                signal_type=SignalType.LIMIT,
                price=bar.close,
                quantity=self.quantity,
                confidence=min(abs(current_z) / 4.0, 1.0),
                rationale=rationale,
            )

        return None
