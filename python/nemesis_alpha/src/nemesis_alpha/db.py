from __future__ import annotations
import os
from datetime import datetime
from typing import Optional

import asyncpg  # type: ignore[import-untyped]


class NemesisDB:
    """Read-only query interface for research and validation."""

    def __init__(self, dsn: Optional[str] = None):
        self._dsn = dsn or os.environ.get(
            "DATABASE_URL",
            "postgres://nemesis:nemesis_dev@localhost:5432/nemesis",
        )
        self._pool: Optional[asyncpg.Pool] = None

    async def connect(self):
        self._pool = await asyncpg.create_pool(self._dsn, min_size=2, max_size=10)

    async def close(self):
        if self._pool:
            await self._pool.close()

    async def get_bars(
        self,
        symbol: str,
        start: datetime,
        end: datetime,
        limit: int = 1000,
    ) -> list[dict]:
        assert self._pool
        rows = await self._pool.fetch(
            """SELECT * FROM bars WHERE symbol = $1 AND time BETWEEN $2 AND $3
               ORDER BY time DESC LIMIT $4""",
            symbol,
            start,
            end,
            limit,
        )
        return [dict(r) for r in rows]

    async def get_audit_log(
        self,
        event_type: Optional[str] = None,
        limit: int = 100,
    ) -> list[dict]:
        assert self._pool
        if event_type:
            rows = await self._pool.fetch(
                "SELECT * FROM audit_log WHERE event_type = $1 ORDER BY receive_ts DESC LIMIT $2",
                event_type,
                limit,
            )
        else:
            rows = await self._pool.fetch(
                "SELECT * FROM audit_log ORDER BY receive_ts DESC LIMIT $1",
                limit,
            )
        return [dict(r) for r in rows]

    async def get_open_orders(
        self,
        symbol: Optional[str] = None,
    ) -> list[dict]:
        assert self._pool
        if symbol:
            rows = await self._pool.fetch(
                """SELECT * FROM orders WHERE symbol = $1
                   AND status NOT IN ('FILLED', 'CANCELED', 'REJECTED')""",
                symbol,
            )
        else:
            rows = await self._pool.fetch(
                """SELECT * FROM orders
                   WHERE status NOT IN ('FILLED', 'CANCELED', 'REJECTED')"""
            )
        return [dict(r) for r in rows]
