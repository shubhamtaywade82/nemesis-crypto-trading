use std::sync::Arc;

use nemesis_core::proto::event_envelope::Payload;
use nemesis_core::{MarketTick, NoopMetrics};
use nemesis_market::{BarBuilder, BarConfig};

#[test]
fn test_volume_bar_deterministic_replay() {
    let metrics: Arc<NoopMetrics> = Arc::new(NoopMetrics);
    let mut builder = BarBuilder::new(
        "BTCUSDT".into(),
        "test".into(),
        BarConfig::VolumeBased { threshold: 100.0 },
    )
    .with_metrics(metrics);

    let ticks = vec![
        MarketTick {
            price: 60000.0,
            quantity: 30.0,
            is_buyer_maker: false,
        },
        MarketTick {
            price: 60001.0,
            quantity: 25.0,
            is_buyer_maker: true,
        },
        MarketTick {
            price: 59999.0,
            quantity: 20.0,
            is_buyer_maker: false,
        },
        MarketTick {
            price: 60002.0,
            quantity: 15.0,
            is_buyer_maker: false,
        },
        MarketTick {
            price: 60000.5,
            quantity: 10.0,
            is_buyer_maker: true,
        },
    ];

    let mut bars = Vec::new();
    for (i, tick) in ticks.iter().enumerate() {
        if let Some(envelope) =
            builder.on_tick(tick, i as u64 + 1, 1700000000000000 + i as i64 * 1000)
        {
            if let Some(Payload::Bar(bar)) = envelope.payload {
                bars.push(bar);
            }
        }
    }

    assert_eq!(
        bars.len(),
        1,
        "Expected exactly 1 bar from 100-volume threshold"
    );
    let bar = &bars[0];
    assert!((bar.open - 60000.0).abs() < f64::EPSILON);
    assert!((bar.high - 60002.0).abs() < f64::EPSILON);
    assert!((bar.low - 59999.0).abs() < f64::EPSILON);
    assert!((bar.close - 60000.5).abs() < f64::EPSILON);
    assert!((bar.volume - 100.0).abs() < f64::EPSILON);
    assert!((bar.buy_volume - 45.0).abs() < f64::EPSILON);
    assert!((bar.sell_volume - 55.0).abs() < f64::EPSILON);
    assert!(!bar.is_corrupted);
}

#[test]
fn test_gap_detection_marks_corrupted() {
    let metrics: Arc<NoopMetrics> = Arc::new(NoopMetrics);
    let mut builder = BarBuilder::new(
        "BTCUSDT".into(),
        "test".into(),
        BarConfig::VolumeBased { threshold: 50.0 },
    )
    .with_metrics(metrics);

    builder.on_tick(
        &MarketTick {
            price: 60000.0,
            quantity: 30.0,
            is_buyer_maker: false,
        },
        1,
        1700000000000000,
    );

    builder.on_tick(
        &MarketTick {
            price: 60001.0,
            quantity: 25.0,
            is_buyer_maker: true,
        },
        5,
        1700000001000000,
    );

    let closed = builder.force_close("test gap").unwrap();
    if let Some(Payload::Bar(bar)) = closed.payload {
        assert!(
            bar.is_corrupted,
            "Bar must be marked corrupted after sequence gap"
        );
    } else {
        panic!("Expected Bar payload");
    }
}
