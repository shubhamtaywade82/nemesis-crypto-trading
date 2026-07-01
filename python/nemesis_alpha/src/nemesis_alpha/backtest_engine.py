from __future__ import annotations
import asyncio
import json
from dataclasses import dataclass, field
from pathlib import Path
from typing import Type

from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.strategy import BaseStrategy
from nemesis_alpha.signals import SignalSide
from nemesis_alpha.proto.envelope_pb2 import EventEnvelope


@dataclass
class TradeRecord:
    entry_time: int
    exit_time: int | None
    side: str
    entry_price: float
    exit_price: float | None
    quantity: float
    pnl: float = 0.0
    is_open: bool = True


@dataclass
class BacktestResult:
    trades: list[TradeRecord]
    bars_processed: int
    corrupted_bars_skipped: int
    signals_generated: int
    equity_curve: list[float]


class BacktestEngine:
    def __init__(self, rust_binary: str, initial_capital: float = 10_000.0):
        self.rust_binary = rust_binary
        self.initial_capital = initial_capital

    async def run(
        self,
        tick_file: Path,
        symbol: str,
        bar_config: dict,
        strategy_cls: Type[BaseStrategy],
        **strategy_kwargs,
    ) -> BacktestResult:
        config_json = json.dumps({
            "symbol": symbol,
            "bar_config": bar_config,
            "input_file": str(tick_file),
        })

        proc = await asyncio.create_subprocess_exec(
            self.rust_binary,
            "--backtest",
            stdin=asyncio.subprocess.PIPE,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )
        assert proc.stdin is not None
        assert proc.stdout is not None
        proc.stdin.write(config_json.encode())
        await proc.stdin.drain()
        proc.stdin.close()

        strategy = strategy_cls(symbol=symbol, **strategy_kwargs)
        trades: list[TradeRecord] = []
        equity_curve: list[float] = [self.initial_capital]
        bars_processed = 0
        corrupted_skipped = 0
        signals_generated = 0
        cash = self.initial_capital
        position: TradeRecord | None = None

        while True:
            len_bytes = await proc.stdout.read(4)
            if not len_bytes or len(len_bytes) < 4:
                break

            frame_len = int.from_bytes(len_bytes, "big")
            frame_data = await proc.stdout.readexactly(frame_len)

            envelope = EventEnvelope()
            envelope.ParseFromString(frame_data)

            if not envelope.HasField("bar"):
                continue

            bar_proto = envelope.bar
            bar = ConsumedBar(
                open=bar_proto.open,
                high=bar_proto.high,
                low=bar_proto.low,
                close=bar_proto.close,
                volume=bar_proto.volume,
                buy_volume=bar_proto.buy_volume,
                sell_volume=bar_proto.sell_volume,
                first_seq=bar_proto.first_seq,
                last_seq=bar_proto.last_seq,
                is_corrupted=bar_proto.is_corrupted,
            )

            if bar.is_corrupted:
                corrupted_skipped += 1
                continue

            bars_processed += 1

            signal_bytes = await strategy.on_bar(bar)
            if signal_bytes is not None:
                signals_generated += 1
                sig_env = EventEnvelope()
                sig_env.ParseFromString(signal_bytes)
                sig = sig_env.signal

                if position is None and sig.side == SignalSide.BUY.value:
                    position = TradeRecord(
                        entry_time=envelope.exchange_ts_us,
                        exit_time=None,
                        side="LONG",
                        entry_price=bar.close,
                        exit_price=None,
                        quantity=sig.quantity,
                    )
                elif position is not None and sig.side == SignalSide.SELL.value:
                    position.exit_time = envelope.exchange_ts_us
                    position.exit_price = bar.close
                    position.pnl = (bar.close - position.entry_price) * position.quantity
                    position.is_open = False
                    cash += position.pnl
                    trades.append(position)
                    position = None

            unrealized = 0.0
            if position is not None:
                unrealized = (bar.close - position.entry_price) * position.quantity
            equity_curve.append(cash + unrealized)

        await proc.wait()

        return BacktestResult(
            trades=trades,
            bars_processed=bars_processed,
            corrupted_bars_skipped=corrupted_skipped,
            signals_generated=signals_generated,
            equity_curve=equity_curve,
        )
