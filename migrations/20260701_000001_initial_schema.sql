CREATE TABLE IF NOT EXISTS orders (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    client_order_id TEXT NOT NULL UNIQUE,
    exchange_id     TEXT,
    symbol          TEXT NOT NULL,
    side            TEXT NOT NULL CHECK (side IN ('BUY', 'SELL')),
    order_type      TEXT NOT NULL CHECK (order_type IN ('LIMIT', 'MARKET')),
    price           NUMERIC(20, 8),
    quantity         NUMERIC(20, 8) NOT NULL,
    filled_qty      NUMERIC(20, 8) NOT NULL DEFAULT 0,
    status          TEXT NOT NULL CHECK (status IN ('PENDING', 'ACCEPTED', 'PARTIALLY_FILLED', 'FILLED', 'CANCELED', 'REJECTED')),
    created_at      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_orders_symbol_status ON orders(symbol, status);
CREATE INDEX idx_orders_created_at ON orders(created_at DESC);

CREATE TABLE IF NOT EXISTS positions (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    symbol          TEXT NOT NULL UNIQUE,
    side            TEXT NOT NULL CHECK (side IN ('LONG', 'SHORT')),
    quantity         NUMERIC(20, 8) NOT NULL,
    avg_entry_price NUMERIC(20, 8) NOT NULL,
    unrealized_pnl  NUMERIC(20, 8) NOT NULL DEFAULT 0,
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS balances (
    id              UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    asset           TEXT NOT NULL,
    free            NUMERIC(20, 8) NOT NULL,
    locked          NUMERIC(20, 8) NOT NULL,
    snapshot_at     TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_balances_asset_time ON balances(asset, snapshot_at DESC);

CREATE TABLE IF NOT EXISTS audit_log (
    id              BIGSERIAL PRIMARY KEY,
    event_id        TEXT NOT NULL UNIQUE,
    source          TEXT NOT NULL,
    symbol          TEXT,
    event_type      TEXT NOT NULL,
    payload         JSONB NOT NULL,
    exchange_ts     TIMESTAMPTZ,
    receive_ts      TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    sequence_num    BIGINT
);

CREATE OR REPLACE FUNCTION prevent_audit_modification()
RETURNS TRIGGER AS $$
BEGIN
    RAISE EXCEPTION 'audit_log is append-only';
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER audit_log_no_update BEFORE UPDATE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();
CREATE TRIGGER audit_log_no_delete BEFORE DELETE ON audit_log
    FOR EACH ROW EXECUTE FUNCTION prevent_audit_modification();

CREATE INDEX idx_audit_event_type ON audit_log(event_type);
CREATE INDEX idx_audit_receive_ts ON audit_log(receive_ts DESC);
CREATE INDEX idx_audit_symbol ON audit_log(symbol);
