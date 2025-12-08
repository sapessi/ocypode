use ocypode::setup_assistant::SetupAssistant;
use ocypode::telemetry::{GameSource, TelemetryData, TelemetryOutput, TireInfo};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::time::Instant;

/// Test that telemetry processing can handle high-frequency data (60+ Hz)
/// This validates that the system can keep up with real-time telemetry streams
#[test]
fn test_high_frequency_telemetry_processing() {
    // Load real telemetry sample
    let file = File::open("telemetry_samples/acc_spa_aston.jsonl")
        .expect("Failed to open telemetry sample");
    let reader = BufReader::new(file);

    let mut telemetry_points = Vec::new();
    for line in reader.lines() {
        let line = line.expect("Failed to read line");
        if let Ok(output) = serde_json::from_str::<TelemetryOutput>(&line) {
            if let TelemetryOutput::DataPoint(data) = output {
                telemetry_points.push(*data);
            }
        }
    }

    assert!(!telemetry_points.is_empty(), "No telemetry points loaded");

    // Simulate 60Hz processing (16.67ms per point)
    let target_time_per_point_us = 16670.0; // microseconds
    let mut assistant = SetupAssistant::new();

    let start = Instant::now();
    for telemetry in &telemetry_points {
        assistant.process_telemetry(telemetry);
    }
    let elapsed = start.elapsed();

    let total_points = telemetry_points.len();
    let avg_time_per_point_us = elapsed.as_micros() as f64 / total_points as f64;

    println!(
        "Processed {} telemetry points in {:?}",
        total_points, elapsed
    );
    println!("Average time per point: {:.2}μs", avg_time_per_point_us);
    println!(
        "Target time per point (60Hz): {:.2}μs",
        target_time_per_point_us
    );

    // Assert that we can process faster than 60Hz
    assert!(
        avg_time_per_point_us < target_time_per_point_us,
        "Processing too slow: {:.2}μs per point (target: {:.2}μs)",
        avg_time_per_point_us,
        target_time_per_point_us
    );
}

/// Test that telemetry cloning is efficient
#[test]
fn test_telemetry_clone_performance() {
    let tire_info = TireInfo {
        left_carcass_temp: 85.0,
        middle_carcass_temp: 85.0,
        right_carcass_temp: 85.0,
        left_surface_temp: 90.0,
        middle_surface_temp: 90.0,
        right_surface_temp: 90.0,
    };

    let telemetry = TelemetryData {
        point_no: 0,
        timestamp_ms: 1000,
        game_source: GameSource::IRacing,
        speed_mps: Some(50.0),
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
    };

    let iterations = 10000;
    let start = Instant::now();
    for _ in 0..iterations {
        let _ = telemetry.clone();
    }
    let elapsed = start.elapsed();

    let avg_clone_time_ns = elapsed.as_nanos() / iterations;
    println!("Average clone time: {}ns", avg_clone_time_ns);

    // Cloning should be very fast (< 1μs)
    assert!(
        avg_clone_time_ns < 1000,
        "Clone too slow: {}ns (target: <1000ns)",
        avg_clone_time_ns
    );
}

/// Test that setup assistant can handle burst processing
#[test]
fn test_burst_processing() {
    let tire_info = TireInfo {
        left_carcass_temp: 85.0,
        middle_carcass_temp: 85.0,
        right_carcass_temp: 85.0,
        left_surface_temp: 90.0,
        middle_surface_temp: 90.0,
        right_surface_temp: 90.0,
    };

    // Create 1000 telemetry points
    let mut telemetry_points = Vec::new();
    for i in 0..1000 {
        telemetry_points.push(TelemetryData {
            point_no: i,
            timestamp_ms: (i * 16) as u128,
            game_source: GameSource::IRacing,
            speed_mps: Some(50.0 + (i as f32 * 0.1)),
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
            rr_tire_info: Some(tire_info.clone()),
            ..Default::default()
        });
    }

    let mut assistant = SetupAssistant::new();

    let start = Instant::now();
    for telemetry in &telemetry_points {
        assistant.process_telemetry(telemetry);
    }
    let elapsed = start.elapsed();

    println!("Burst processed 1000 points in {:?}", elapsed);
    println!(
        "Average: {:.2}μs per point",
        elapsed.as_micros() as f64 / 1000.0
    );

    // Should process 1000 points in less than 100ms
    assert!(
        elapsed.as_millis() < 100,
        "Burst processing too slow: {:?}ms (target: <100ms)",
        elapsed.as_millis()
    );
}

/// Test memory efficiency - ensure we don't leak or accumulate excessive memory
#[test]
fn test_memory_efficiency() {
    let tire_info = TireInfo {
        left_carcass_temp: 85.0,
        middle_carcass_temp: 85.0,
        right_carcass_temp: 85.0,
        left_surface_temp: 90.0,
        middle_surface_temp: 90.0,
        right_surface_temp: 90.0,
    };

    let mut assistant = SetupAssistant::new();

    // Process 10,000 points to simulate a long session
    for i in 0..10000 {
        let telemetry = TelemetryData {
            point_no: i,
            timestamp_ms: (i * 16) as u128,
            game_source: GameSource::IRacing,
            speed_mps: Some(50.0),
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
            rr_tire_info: Some(tire_info.clone()),
            ..Default::default()
        };
        assistant.process_telemetry(&telemetry);
    }

    // Verify findings are bounded
    let findings = assistant.get_findings();
    assert!(
        findings.len() < 100,
        "Too many findings accumulated: {} (should be bounded)",
        findings.len()
    );

    println!("After 10,000 points: {} unique findings", findings.len());
}
