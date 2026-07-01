from __future__ import annotations
import asyncio
import json
import subprocess
from dataclasses import dataclass, field
from pathlib import Path
from typing import AsyncIterator, Iterator

from nemesis_alpha.consumer import ConsumedBar
from nemesis_alpha.proto.envelope_pb2 import EventEnvelope


@dataclass
class BacktestResult:
    total_return: float = 0.0
    sharpe_ratio: float = 0.0
    max_drawdown: float = 0.0
    total_trades: int = 0
    win_rate: float = 0.0
    avg_slippage_bps: float = 0.0
    bars_processed: int = 0
    corrupted_bars: int = 0


class BacktestHarness:
    """Deterministic backtester that replays raw ticks through the Rust BarBuilder."""

    def __init__(self, rust_binary_path: str, symbol: str, bar_config: dict):
        self.rust_binary_path = rust_binary_path
        self.symbol = symbol
        self.bar_config = bar_config

    def replay_ticks(self, tick_file: Path) -> Iterator[ConsumedBar]:
        config_json = json.dumps({
            "symbol": self.symbol,
            "bar_config": self.bar_config,
            "input_file": str(tick_file),
            "output_format": "protobuf-binary",
        })

        proc = subprocess.Popen(
            [self.rust_binary_path, "--backtest"],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
        )

        assert proc.stdin is not None
        assert proc.stdout is not None
        assert proc.stderr is not None

        proc.stdin.write(config_json.encode())
        proc.stdin.flush()
        proc.stdin.close()

        while True:
            len_bytes = proc.stdout.read(4)
            if not len_bytes or len(len_bytes) < 4:
                break

            frame_len = int.from_bytes(len_bytes, "big")
            frame_data = proc.stdout.read(frame_len)

            if len(frame_data) < frame_len:
                break

            envelope = EventEnvelope()
            envelope.ParseFromString(frame_data)

            if envelope.HasField("bar"):
                bar = envelope.bar
                yield ConsumedBar(
                    open=bar.open,
                    high=bar.high,
                    low=bar.low,
                    close=bar.close,
                    volume=bar.volume,
                    buy_volume=bar.buy_volume,
                    sell_volume=bar.sell_volume,
                    first_seq=bar.first_seq,
                    last_seq=bar.last_seq,
                    is_corrupted=bar.is_corrupted,
                )

        proc.wait()
        if proc.returncode != 0:
            stderr = proc.stderr.read().decode()
            raise RuntimeError(f"Backtest binary failed: {stderr}")

    def run(self, tick_file: Path, strategy_factory) -> BacktestResult:
        strategy = strategy_factory(self.symbol)
        bars_processed = 0
        corrupted_bars = 0
        trades: list[dict] = []

        for bar in self.replay_ticks(tick_file):
            bars_processed += 1
            if bar.is_corrupted:
                corrupted_bars += 1

        return BacktestResult(
            bars_processed=bars_processed,
            corrupted_bars=corrupted_bars,
            total_trades=len(trades),
        )
