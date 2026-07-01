from __future__ import annotations
import logging
from typing import AsyncIterator

from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.signals import SignalEmitter, SignalSide, SignalType

logger = logging.getLogger(__name__)


class BaseStrategy:
    """Base class for all Nemesis strategies.

    Subclasses implement on_bar() to generate signals.
    """

    def __init__(self, symbol: str):
        self.symbol = symbol
        self.emitter = SignalEmitter(source=f"strategy-{symbol}")
        self._bars: list[ConsumedBar] = []

    async def run(self, bar_stream: AsyncIterator[ConsumedBar]) -> AsyncIterator[bytes]:
        async for bar in bar_stream:
            if bar.is_corrupted:
                logger.warning(
                    "Skipping corrupted bar: seq %s-%s",
                    bar.first_seq,
                    bar.last_seq,
                )
                continue

            self._bars.append(bar)

            if len(self._bars) > 500:
                self._bars = self._bars[-500:]

            signal_bytes = await self.on_bar(bar)
            if signal_bytes is not None:
                yield signal_bytes

    async def on_bar(self, bar: ConsumedBar) -> bytes | None:
        raise NotImplementedError


class ExampleMomentumStrategy(BaseStrategy):
    """Simple momentum strategy for demonstration."""

    def __init__(self, symbol: str, lookback: int = 20, threshold: float = 0.004):
        super().__init__(symbol)
        self.lookback = lookback
        self.threshold = threshold
        self._bar_history: list[ConsumedBar] = []

    async def on_bar(self, bar: ConsumedBar) -> bytes | None:
        self._bar_history.append(bar)
        if len(self._bar_history) > max(self.lookback * 2, 500):
            self._bar_history = self._bar_history[-max(self.lookback * 2, 500):]

        if len(self._bar_history) < self.lookback:
            return None

        past_close = self._bar_history[-self.lookback].close
        momentum = (bar.close - past_close) / past_close

        if momentum > self.threshold:
            return self.emitter.build_signal(
                symbol=self.symbol,
                side=SignalSide.BUY,
                signal_type=SignalType.MARKET,
                price=None,
                quantity=0.001,
                confidence=min(abs(momentum) / (self.threshold * 2), 1.0),
                rationale=f"Momentum {momentum:.4f} over {self.lookback} bars",
            )
        elif momentum < -self.threshold:
            return self.emitter.build_signal(
                symbol=self.symbol,
                side=SignalSide.SELL,
                signal_type=SignalType.MARKET,
                price=None,
                quantity=0.001,
                confidence=min(abs(momentum) / (self.threshold * 2), 1.0),
                rationale=f"Negative momentum {momentum:.4f} over {self.lookback} bars",
            )

        return None
