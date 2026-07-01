from __future__ import annotations
import asyncio
import logging
from typing import AsyncIterator

from google.protobuf.message import DecodeError
from pydantic import BaseModel, ValidationError

from nemesis_alpha.proto.envelope_pb2 import EventEnvelope

logger = logging.getLogger(__name__)


class ConsumedBar(BaseModel):
    open: float
    high: float
    low: float
    close: float
    volume: float
    buy_volume: float
    sell_volume: float
    first_seq: int
    last_seq: int
    is_corrupted: bool

    @property
    def delta(self) -> float:
        return self.buy_volume - self.sell_volume


class EventConsumer:
    """Consumes serialized EventEnvelopes from the Rust publisher."""

    def __init__(self, queue: asyncio.Queue[bytes]):
        self._queue = queue

    async def stream_bars(self) -> AsyncIterator[ConsumedBar]:
        while True:
            raw = await self._queue.get()
            envelope = EventEnvelope()

            try:
                envelope.ParseFromString(raw)
            except DecodeError as e:
                logger.error("Proto decode failed: %s", e)
                continue

            if not envelope.HasField("bar"):
                continue

            bar = envelope.bar
            try:
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
            except ValidationError as e:
                logger.error("Bar validation failed: %s", e)
                continue
