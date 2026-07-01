#!/usr/bin/env python3
"""Download ETHUSDT aggTrades in 12-hour chunks, writing a single chronological CSV."""
import csv
import sys
import time
from pathlib import Path
from datetime import datetime, timezone, timedelta
import requests

BINANCE_FAPI = "https://fapi.binance.com/fapi/v1/aggTrades"
MAX_LIMIT = 1000
CHUNK_HOURS = 12
RATE_LIMIT_PAUSE = 0.15


def ms_to_dt(ms: int) -> str:
    return datetime.fromtimestamp(ms / 1000, tz=timezone.utc).isoformat()


def download_chunk(
    symbol: str,
    output_path: Path,
    start_ms: int,
    end_ms: int,
    append: bool = False,
) -> int:
    written = 0
    from_id = None

    # Find first trade ID at or after start_ms
    resp = requests.get(
        BINANCE_FAPI,
        params={"symbol": symbol, "startTime": start_ms, "limit": 1},
        timeout=30,
    )
    resp.raise_for_status()
    trades = resp.json()
    if not trades:
        print(f"  No trades at {ms_to_dt(start_ms)}")
        return 0
    from_id = trades[0]["a"]

    mode = "a" if append else "w"
    with open(output_path, mode, newline="") as f:
        writer = csv.writer(f)

        while True:
            params: dict = {"symbol": symbol, "limit": MAX_LIMIT, "fromId": from_id}

            try:
                resp = requests.get(BINANCE_FAPI, params=params, timeout=30)
                if resp.status_code == 429:
                    retry_after = int(resp.headers.get("Retry-After", 10))
                    print(f"\n  429: retry {retry_after}s")
                    time.sleep(retry_after)
                    continue
                resp.raise_for_status()
                trades = resp.json()
            except Exception as e:
                print(f"\n  Error: {e}, retry 5s")
                time.sleep(5)
                continue

            if not trades:
                break

            for t in trades:
                ts_ms = int(t["T"])
                if ts_ms >= end_ms:
                    return written
                writer.writerow([ts_ms * 1000, t["p"], t["q"], str(t["m"]).lower()])
                written += 1
                from_id = t["a"] + 1

            sys.stdout.write(
                f"\r  {written:>8,} ticks | last: {ms_to_dt(trades[-1]['T'])}"
            )
            sys.stdout.flush()

            if len(trades) < MAX_LIMIT:
                break
            time.sleep(RATE_LIMIT_PAUSE)

    print()
    return written


def main():
    symbol = "ETHUSDT"
    output = (
        Path(__file__).resolve().parent.parent.parent.parent
        / "data"
        / "ticks_ethusdt_2026.csv"
    )
    output.parent.mkdir(parents=True, exist_ok=True)

    # Overwrite existing file with a fresh download
    if output.exists():
        bak = output.with_suffix(".csv.bak")
        output.rename(bak)
        print(f"Backed up existing file -> {bak.name}")

    start = datetime(2026, 6, 1, tzinfo=timezone.utc)
    end = datetime(2026, 6, 17, tzinfo=timezone.utc)
    total_seconds = int((end - start).total_seconds())
    print(f"Downloading {symbol} {start.date()} -> {end.date()} "
          f"({total_seconds // 3600}h)")

    grand_total = 0
    chunk_idx = 0
    current = start

    while current < end:
        chunk_end = min(current + timedelta(hours=CHUNK_HOURS), end)
        start_ms = int(current.timestamp() * 1000)
        end_ms = int(chunk_end.timestamp() * 1000)

        print(f"\n[{chunk_idx}] {current.isoformat()} -> {chunk_end.isoformat()}")
        count = download_chunk(
            symbol, output, start_ms, end_ms, append=(chunk_idx > 0)
        )
        grand_total += count
        print(f"       {count:>8,} ticks (total: {grand_total:>8,})")

        current = chunk_end
        chunk_idx += 1

    size_mb = output.stat().st_size / 1024 / 1024
    print(f"\nDone: {grand_total:,} ticks, {size_mb:.1f} MB -> {output}")


if __name__ == "__main__":
    main()
