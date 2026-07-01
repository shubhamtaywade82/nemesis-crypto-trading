CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

CREATE TABLE IF NOT EXISTS bars (
    symbol          TEXT NOT NULL,
    time            TIMESTAMPTZ NOT NULL,
    open            NUMERIC(20, 8) NOT NULL,
    high            NUMERIC(20, 8) NOT NULL,
    low             NUMERIC(20, 8) NOT NULL,
    close           NUMERIC(20, 8) NOT NULL,
    volume          NUMERIC(20, 8) NOT NULL,
    buy_volume      NUMERIC(20, 8) NOT NULL,
    sell_volume     NUMERIC(20, 8) NOT NULL,
    delta           NUMERIC(20, 8) NOT NULL,
    first_seq       BIGINT NOT NULL,
    last_seq        BIGINT NOT NULL,
    is_corrupted    BOOLEAN NOT NULL DEFAULT FALSE
);

SELECT create_hypertable('bars', 'time', if_not_exists => TRUE);
CREATE INDEX idx_bars_symbol_time ON bars(symbol, time DESC);
