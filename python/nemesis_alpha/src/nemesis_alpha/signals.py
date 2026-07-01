from __future__ import annotations
import time
import uuid
from enum import IntEnum

from google.protobuf.message import EncodeError

from nemesis_alpha.proto.envelope_pb2 import EventEnvelope, TradeSignal


class SignalSide(IntEnum):
    BUY = 0
    SELL = 1


class SignalType(IntEnum):
    LIMIT = 0
    MARKET = 1


class SignalEmitter:
    """Serializes TradeSignals into EventEnvelopes for the Rust execution engine."""

    def __init__(self, source: str = "python-alpha"):
        self._source = source

    def build_signal(
        self,
        symbol: str,
        side: SignalSide,
        signal_type: SignalType,
        price: float | None,
        quantity: float,
        confidence: float,
        rationale: str = "",
    ) -> bytes:
        if confidence < 0.0 or confidence > 1.0:
            raise ValueError(f"Confidence must be [0, 1], got {confidence}")
        if quantity <= 0:
            raise ValueError(f"Quantity must be positive, got {quantity}")
        if signal_type == SignalType.LIMIT and price is None:
            raise ValueError("LIMIT orders require a price")

        signal = TradeSignal(
            side=side.value,
            signal_type=signal_type.value,
            price=price if price is not None else 0.0,
            quantity=quantity,
            confidence=confidence,
            rationale=rationale,
        )

        envelope = EventEnvelope(
            event_id=str(uuid.uuid7()),
            source=self._source,
            symbol=symbol,
            exchange_ts_us=int(time.time() * 1_000_000),
            receive_ts_us=int(time.time() * 1_000_000),
            sequence_num=0,
        )
        envelope.signal.CopyFrom(signal)

        try:
            return envelope.SerializeToString()
        except EncodeError as e:
            raise ValueError(f"Failed to serialize signal: {e}") from e
