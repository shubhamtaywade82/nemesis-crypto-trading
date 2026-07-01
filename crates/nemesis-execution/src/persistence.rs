use nemesis_core::BarClosed;
use sqlx::PgPool;
use tracing::{debug, error};

pub struct PersistenceWriter {
    pool: PgPool,
}

impl PersistenceWriter {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn write_bars(&self, symbol: &str, bars: &[BarClosed]) -> anyhow::Result<()> {
        if bars.is_empty() {
            return Ok(());
        }

        for bar in bars {
            let result = sqlx::query(
                r#"INSERT INTO bars (symbol, time, open, high, low, close, volume,
                   buy_volume, sell_volume, delta, first_seq, last_seq, is_corrupted)
                   VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
                   ON CONFLICT DO NOTHING"#,
            )
            .bind(symbol)
            .bind(chrono::Utc::now())
            .bind(bar.open)
            .bind(bar.high)
            .bind(bar.low)
            .bind(bar.close)
            .bind(bar.volume)
            .bind(bar.buy_volume)
            .bind(bar.sell_volume)
            .bind(bar.buy_volume - bar.sell_volume)
            .bind(bar.first_seq as i64)
            .bind(bar.last_seq as i64)
            .bind(bar.is_corrupted)
            .execute(&self.pool)
            .await;

            if let Err(e) = result {
                error!(symbol, ?e, "Failed to write bar");
            }
        }

        debug!(symbol, count = bars.len(), "Wrote bars batch");
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub async fn write_audit(
        &self,
        event_id: &str,
        source: &str,
        symbol: Option<&str>,
        event_type: &str,
        payload: serde_json::Value,
        exchange_ts: Option<chrono::DateTime<chrono::Utc>>,
        sequence_num: Option<i64>,
    ) -> anyhow::Result<()> {
        sqlx::query(
            r#"INSERT INTO audit_log (event_id, source, symbol, event_type, payload,
               exchange_ts, sequence_num) VALUES ($1, $2, $3, $4, $5, $6, $7)"#,
        )
        .bind(event_id)
        .bind(source)
        .bind(symbol)
        .bind(event_type)
        .bind(payload)
        .bind(exchange_ts)
        .bind(sequence_num)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
