use std::sync::Arc;

use nemesis_core::proto::event_envelope::Payload;
use nemesis_core::MarketTick;
use nemesis_core::NoopMetrics;
use nemesis_market::{BarBuilder, BarConfig};

#[test]
fn test_live_vs_replay_determinism() {
    let ticks = [
        MarketTick {
            price: 60000.0,
            quantity: 50.0,
            is_buyer_maker: false,
        },
        MarketTick {
            price: 60001.0,
            quantity: 30.0,
            is_buyer_maker: true,
        },
        MarketTick {
            price: 59999.0,
            quantity: 25.0,
            is_buyer_maker: false,
        },
    ];

    let build_bars = || {
        let metrics = Arc::new(NoopMetrics);
        let mut builder = BarBuilder::new(
            "BTCUSDT".into(),
            "test".into(),
            BarConfig::VolumeBased { threshold: 100.0 },
        )
        .with_metrics(metrics);

        let mut bars = Vec::new();
        for (i, tick) in ticks.iter().enumerate() {
            if let Some(env) =
                builder.on_tick(tick, (i + 1) as u64, 1700000000000000 + i as i64 * 1000)
            {
                if let Some(Payload::Bar(b)) = env.payload {
                    bars.push((b.open, b.high, b.low, b.close, b.volume));
                }
            }
        }
        bars
    };

    let run1 = build_bars();
    let run2 = build_bars();
    assert_eq!(
        run1, run2,
        "BarBuilder must be deterministic across identical runs"
    );
}
