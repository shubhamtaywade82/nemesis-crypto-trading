#!/usr/bin/env python3
"""Download historical aggTrades from Binance Futures REST API.

Uses fromId pagination for O(n) throughput — no time-window gaps.
"""
from __future__ import annotations
import argparse
import csv
import time
from pathlib import Path
from datetime import datetime
import requests
from tqdm import tqdm

BINANCE_FAPI = "https://fapi.binance.com/fapi/v1/aggTrades"
MAX_LIMIT = 1000
RATE_LIMIT_PAUSE = 0.15  # ~6.6 req/s


def download_ticks(
    symbol: str,
    start_str: str,
    end_str: str,
    output_path: Path,
) -> None:
    start_ms = int(datetime.fromisoformat(start_str).timestamp() * 1000)
    end_ms = int(datetime.fromisoformat(end_str).timestamp() * 1000)

    output_path.parent.mkdir(parents=True, exist_ok=True)
    total_written = 0
    estimate = 50_000_000  # rough guess for 90d BTCUSDT

    # First request: find the earliest trade >= start_ms
    params: dict[str, str | int] = {
        "symbol": symbol,
        "startTime": start_ms,
        "limit": 1,
    }
    resp = requests.get(BINANCE_FAPI, params=params, timeout=30)
    resp.raise_for_status()
    seed = resp.json()
    if not seed:
        print(f"No trades found after {start_str}")
        return
    next_id: int = seed[0]["a"]

    output_path.parent.mkdir(parents=True, exist_ok=True)
    pbar = tqdm(total=estimate, desc=f"Downloading {symbol}", unit="ticks")

    with open(output_path, "w", newline="") as f:
        writer = csv.writer(f)

        while True:
            params = {
                "symbol": symbol,
                "fromId": next_id,
                "limit": MAX_LIMIT,
            }

            for attempt in range(5):
                try:
                    resp = requests.get(BINANCE_FAPI, params=params, timeout=30)
                    if resp.status_code == 429:
                        retry_after = int(resp.headers.get("Retry-After", 10))
                        tqdm.write(f"\nRate limited. Waiting {retry_after}s...")
                        time.sleep(retry_after)
                        continue
                    resp.raise_for_status()
                    trades = resp.json()
                    break
                except Exception as e:
                    if attempt == 4:
                        raise RuntimeError(f"Failed after 5 attempts: {e}") from e
                    time.sleep(2 ** attempt)

            if not trades:
                break

            keep = []
            for t in trades:
                if int(t["T"]) > end_ms:
                    break
                keep.append(t)

            if not keep:
                break

            for t in keep:
                ts_us = int(t["T"]) * 1000
                writer.writerow([ts_us, t["p"], t["q"], str(t["m"]).lower()])
                total_written += 1
                pbar.update(1)

            # Paginate forward by trade ID
            next_id = keep[-1]["a"] + 1

            # If fewer than MAX_LIMIT returned, we've exhausted the range
            if len(trades) < MAX_LIMIT:
                break

            time.sleep(RATE_LIMIT_PAUSE)

    pbar.total = total_written
    pbar.refresh()
    pbar.close()

    print(f"Wrote {total_written:,} ticks to {output_path}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Download Binance Futures aggTrades")
    parser.add_argument("--symbol", default="BTCUSDT")
    parser.add_argument("--start", required=True, help="ISO format: 2026-04-01T00:00:00")
    parser.add_argument("--end", required=True, help="ISO format: 2026-07-01T00:00:00")
    parser.add_argument("--output", default="data/ticks_btcusdt_2026.csv")
    args = parser.parse_args()

    download_ticks(args.symbol, args.start, args.end, Path(args.output))
