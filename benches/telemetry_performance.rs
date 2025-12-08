use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ocypode::setup_assistant::SetupAssistant;
use ocypode::telemetry::{GameSource, TelemetryData, TireInfo};
use std::time::Duration;

fn create_sample_telemetry(point_no: usize) -> TelemetryData {
    let tire_info = TireInfo {
        left_carcass_temp: 85.0,
        middle_carcass_temp: 85.0,
        right_carcass_temp: 85.0,
        left_surface_temp: 90.0,
        middle_surface_temp: 90.0,
        right_surface_temp: 90.0,
    };

    TelemetryData {
        point_no,
        timestamp_ms: (point_no * 16) as u128, // ~60Hz
        game_source: GameSource::IRacing,
        speed_mps: Some(50.0 + (point_no as f32 * 0.1)),
        throttle: Some(0.8),
        brake: Some(0.2),
        steering_pct: Some(0.3),
        gear: Some(3),
        engine_rpm: Some(5000.0),
        shift_point_rpm: Some(6000.0),
        yaw_rate_rps: Some(0.5),
        pitch_rad: Some(0.1),
        roll_rad: Some(0.05),
        is_abs_active: Some(false),
        lf_tire_info: Some(tire_info.clone()),
        rf_tire_info: Some(tire_info.clone()),
        lr_tire_info: Some(tire_info.clone()),
        rr_tire_info: Some(tire_info),
        ..Default::default()
    }
}

fn bench_telemetry_clone(c: &mut Criterion) {
    let mut group = c.benchmark_group("telemetry_operations");

    let telemetry = create_sample_telemetry(0);

    group.bench_function("clone_telemetry", |b| {
        b.iter(|| black_box(telemetry.clone()));
    });

    group.bench_function("box_telemetry", |b| {
        b.iter(|| black_box(Box::new(telemetry.clone())));
    });

    group.finish();
}

fn bench_setup_assistant_processing(c: &mut Criterion) {
    let mut group = c.benchmark_group("setup_assistant");

    group.bench_function("process_single_telemetry", |b| {
        let telemetry = create_sample_telemetry(0);
        let mut assistant = SetupAssistant::new();
        b.iter(|| {
            assistant.process_telemetry(black_box(&telemetry));
        });
    });

    group.bench_function("process_100_telemetry_points", |b| {
        b.iter(|| {
            let mut assistant = SetupAssistant::new();
            for i in 0..100 {
                let telemetry = create_sample_telemetry(i);
                assistant.process_telemetry(&telemetry);
            }
        });
    });

    group.bench_function("process_1000_telemetry_points", |b| {
        b.iter(|| {
            let mut assistant = SetupAssistant::new();
            for i in 0..1000 {
                let telemetry = create_sample_telemetry(i);
                assistant.process_telemetry(&telemetry);
            }
        });
    });

    group.finish();
}

fn bench_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("serialization");

    let telemetry = create_sample_telemetry(0);

    group.bench_function("serialize_telemetry", |b| {
        b.iter(|| black_box(serde_json::to_string(&telemetry).unwrap()));
    });

    let json = serde_json::to_string(&telemetry).unwrap();
    group.bench_function("deserialize_telemetry", |b| {
        b.iter(|| black_box(serde_json::from_str::<TelemetryData>(&json).unwrap()));
    });

    group.finish();
}

criterion_group! {
    name = benches;
    config = Criterion::default()
        .measurement_time(Duration::from_secs(10))
        .sample_size(100);
    targets = bench_telemetry_clone, bench_setup_assistant_processing, bench_serialization
}
criterion_main!(benches);
