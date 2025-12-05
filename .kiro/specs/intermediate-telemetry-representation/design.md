# Design Document: Intermediate Telemetry Representation

## Overview

This design introduces a unified intermediate telemetry representation that decouples analyzers from game-specific implementations. The current architecture relies on the simetry library's `Moment` trait, which provides limited access to telemetry data through its base interface. Game-specific data requires unsafe downcasting to concrete types (`simetry::iracing::SimState` or `simetry::assetto_corsa_competizione::SimState`), which is error-prone and makes the codebase harder to maintain.

The new design replaces this approach with a safe, game-agnostic intermediate representation called `TelemetryData`. Each game-specific producer will be responsible for converting its native telemetry format into `TelemetryData`, eliminating unsafe code and providing a clean separation of concerns.

## Architecture

### Current Architecture Issues

1. **Unsafe downcasting**: `SerializableTelemetry::from_moment_boxed()` uses raw pointer casting to convert `Box<dyn Moment>` to game-specific types
2. **Limited trait access**: The base `Moment` trait doesn't expose steering angle, yaw rate, or other critical fields needed by analyzers
3. **Tight coupling**: Analyzers receive `&dyn Moment` but cannot access game-specific fields without unsafe code
4. **Dual data structures**: Both `SerializableTelemetry` and `Moment` trait objects are used throughout the codebase

### New Architecture

```
┌─────────────────────┐
│  Game (iRacing/ACC) │
└──────────┬──────────┘
           │ simetry::Moment
           ▼
┌─────────────────────┐
│  Producer           │
│  - IRacing          │
│  - ACC              │
│  - Mock             │
└──────────┬──────────┘
           │ TelemetryData (intermediate representation)
           ▼
┌─────────────────────┐
│  Collector          │
└──────────┬──────────┘
           │ TelemetryData
           ├──────────────────┐
           ▼                  ▼
┌─────────────────────┐  ┌─────────────────────┐
│  Analyzers          │  │  Writer/UI          │
│  - Slip             │  │                     │
│  - Wheelspin        │  │                     │
│  - Scrub            │  │                     │
│  - etc.             │  │                     │
└─────────────────────┘  └─────────────────────┘
```

### Key Changes

1. **TelemetryProducer trait returns TelemetryData**: The `telemetry()` method will return `Result<TelemetryData, OcypodeError>` instead of `Result<Box<dyn Moment>, OcypodeError>`
2. **TelemetryAnalyzer trait accepts TelemetryData**: The `analyze()` method will accept `&TelemetryData` instead of `&dyn Moment`
3. **Game-specific conversion**: Each producer implementation handles conversion from simetry types to `TelemetryData`
4. **Single data structure**: `TelemetryData` replaces both `SerializableTelemetry` and the use of `Moment` trait objects

## Components and Interfaces

### TelemetryData Struct

The core intermediate representation that captures all possible telemetry data points from supported games.

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TelemetryData {
    // Metadata
    pub point_no: usize,
    pub timestamp_ms: u128,
    pub game_source: GameSource,
    
    // Vehicle state
    pub gear: Option<i8>,
    pub speed_mps: Option<f32>,
    pub engine_rpm: Option<f32>,
    pub max_engine_rpm: Option<f32>,
    pub shift_point_rpm: Option<f32>,
    
    // Inputs
    pub throttle: Option<f32>,
    pub brake: Option<f32>,
    pub clutch: Option<f32>,
    pub steering_angle_rad: Option<f32>,  // Renamed from 'steering' for clarity
    pub steering_pct: Option<f32>,
    
    // Position and lap data
    pub lap_distance_m: Option<f32>,     // Renamed from 'lap_distance' for clarity
    pub lap_distance_pct: Option<f32>,
    pub lap_number: Option<u32>,
    
    // Timing
    pub last_lap_time_s: Option<f32>,
    pub best_lap_time_s: Option<f32>,
    
    // Flags and states
    pub is_pit_limiter_engaged: Option<bool>,
    pub is_in_pit_lane: Option<bool>,
    pub is_abs_active: Option<bool>,     // Renamed from 'abs_active' for consistency
    
    // GPS coordinates (iRacing only)
    pub latitude_deg: Option<f32>,       // Renamed from 'lat' for clarity
    pub longitude_deg: Option<f32>,      // Renamed from 'lon' for clarity
    
    // Acceleration
    pub lateral_accel_mps2: Option<f32>, // Renamed from 'lat_accel' for clarity
    pub longitudinal_accel_mps2: Option<f32>, // Renamed from 'lon_accel' for clarity
    
    // Orientation
    pub pitch_rad: Option<f32>,          // Renamed from 'pitch' for clarity
    pub pitch_rate_rps: Option<f32>,     // Renamed from 'pitch_rate' for clarity
    pub roll_rad: Option<f32>,           // Renamed from 'roll' for clarity
    pub roll_rate_rps: Option<f32>,      // Renamed from 'roll_rate' for clarity
    pub yaw_rad: Option<f32>,            // Renamed from 'yaw' for clarity
    pub yaw_rate_rps: Option<f32>,       // Renamed from 'yaw_rate' for clarity
    
    // Tire data
    pub lf_tire_info: Option<TireInfo>,
    pub rf_tire_info: Option<TireInfo>,
    pub lr_tire_info: Option<TireInfo>,
    pub rr_tire_info: Option<TireInfo>,
    
    // Analyzer annotations
    pub annotations: Vec<TelemetryAnnotation>,
}
```

### Updated TelemetryProducer Trait

```rust
pub trait TelemetryProducer {
    fn start(&mut self) -> Result<(), OcypodeError>;
    fn session_info(&mut self) -> Result<SessionInfo, OcypodeError>;
    
    // Changed return type from Box<dyn Moment> to TelemetryData
    fn telemetry(&mut self) -> Result<TelemetryData, OcypodeError>;
    
    fn game_source(&self) -> GameSource;
}
```

### Updated TelemetryAnalyzer Trait

```rust
pub trait TelemetryAnalyzer {
    // Changed parameter from &dyn Moment to &TelemetryData
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        session_info: &SessionInfo,
    ) -> Vec<TelemetryAnnotation>;
}
```

### Game-Specific Conversion Functions

Each producer will implement conversion from simetry types to `TelemetryData`:

```rust
impl TelemetryData {
    #[cfg(windows)]
    pub fn from_iracing_state(
        state: &simetry::iracing::SimState,
        point_no: usize,
    ) -> Self {
        // Extract all available iRacing fields
        // Use simetry's Moment trait methods plus iRacing-specific accessors
    }
    
    #[cfg(windows)]
    pub fn from_acc_state(
        state: &simetry::assetto_corsa_competizione::SimState,
        point_no: usize,
    ) -> Self {
        // Extract all available ACC fields
        // Use simetry's Moment trait methods plus ACC-specific accessors
    }
    
    pub fn from_serializable(data: SerializableTelemetry) -> Self {
        // Convert legacy SerializableTelemetry to TelemetryData
        // Used by MockTelemetryProducer for loading saved files
    }
}
```

## Data Models

### TelemetryData Field Mapping

#### iRacing Data Sources

| Field | Source | Notes |
|-------|--------|-------|
| gear | `Moment::vehicle_gear()` | Base trait |
| speed_mps | `Moment::vehicle_velocity()` | Base trait, convert to m/s |
| engine_rpm | `Moment::vehicle_engine_rotation_speed()` | Base trait, convert to RPM |
| max_engine_rpm | `Moment::vehicle_max_engine_rotation_speed()` | Base trait, convert to RPM |
| shift_point_rpm | `Moment::shift_point()` | Base trait, convert to RPM |
| throttle | `Moment::pedals().throttle` | Base trait |
| brake | `Moment::pedals().brake` | Base trait |
| clutch | `Moment::pedals().clutch` | Base trait |
| steering_angle_rad | iRacing telemetry var | Game-specific, needs research |
| steering_pct | iRacing telemetry var | Game-specific, needs research |
| lap_distance_m | iRacing telemetry var | Game-specific |
| lap_distance_pct | iRacing telemetry var | Game-specific |
| lap_number | iRacing telemetry var | Game-specific |
| last_lap_time_s | iRacing telemetry var | Game-specific |
| best_lap_time_s | iRacing telemetry var | Game-specific |
| is_pit_limiter_engaged | `Moment::is_pit_limiter_engaged()` | Base trait |
| is_in_pit_lane | `Moment::is_vehicle_in_pit_lane()` | Base trait |
| is_abs_active | iRacing telemetry var | Game-specific |
| latitude_deg | iRacing telemetry var | Game-specific |
| longitude_deg | iRacing telemetry var | Game-specific |
| lateral_accel_mps2 | iRacing telemetry var | Game-specific |
| longitudinal_accel_mps2 | iRacing telemetry var | Game-specific |
| pitch_rad | iRacing telemetry var | Game-specific |
| pitch_rate_rps | iRacing telemetry var | Game-specific |
| roll_rad | iRacing telemetry var | Game-specific |
| roll_rate_rps | iRacing telemetry var | Game-specific |
| yaw_rad | iRacing telemetry var | Game-specific |
| yaw_rate_rps | iRacing telemetry var | Game-specific |
| tire_info | iRacing telemetry var | Game-specific |

#### ACC Data Sources

| Field | Source | Notes |
|-------|--------|-------|
| gear | `Moment::vehicle_gear()` | Base trait |
| speed_mps | `Moment::vehicle_velocity()` | Base trait |
| engine_rpm | `Moment::vehicle_engine_rotation_speed()` | Base trait |
| max_engine_rpm | `Moment::vehicle_max_engine_rotation_speed()` | Base trait |
| shift_point_rpm | `Moment::shift_point()` | Base trait |
| throttle | `state.physics.gas` | Direct field access |
| brake | `state.physics.brake` | Direct field access |
| clutch | `state.physics.clutch` | Direct field access |
| steering_angle_rad | `state.physics.steer_angle` | Direct field access |
| steering_pct | `state.physics.steer_angle` | Same as angle (normalized) |
| lap_distance_m | None | Not available in ACC |
| lap_distance_pct | `state.graphics.normalized_car_position` | Direct field access |
| lap_number | `state.graphics.completed_laps` | Direct field access |
| last_lap_time_s | `state.graphics.lap_timing.last.millis / 1000` | Convert from ms |
| best_lap_time_s | `state.graphics.lap_timing.best.millis / 1000` | Convert from ms |
| is_pit_limiter_engaged | `Moment::is_pit_limiter_engaged()` | Base trait |
| is_in_pit_lane | `Moment::is_vehicle_in_pit_lane()` | Base trait |
| is_abs_active | `state.physics.abs > 0.0` | Derived from ABS value |
| latitude_deg | None | Not available in ACC |
| longitude_deg | None | Not available in ACC |
| lateral_accel_mps2 | None | Needs research |
| longitudinal_accel_mps2 | None | Needs research |
| pitch_rad | `state.physics.pitch` | Direct field access |
| pitch_rate_rps | None | Not available in ACC |
| roll_rad | `state.physics.roll` | Direct field access |
| roll_rate_rps | None | Not available in ACC |
| yaw_rad | `state.physics.heading` | Direct field access |
| yaw_rate_rps | None | Needs research |
| tire_info | `state.physics` | Needs research for temp fields |

### TireInfo Structure

Remains unchanged from current implementation:

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TireInfo {
    pub left_carcass_temp: f32,
    pub middle_carcass_temp: f32,
    pub right_carcass_temp: f32,
    pub left_surface_temp: f32,
    pub middle_surface_temp: f32,
    pub right_surface_temp: f32,
}
```


## Correctness Properties

*A property is a characteristic or behavior that should hold true across all valid executions of a system-essentially, a formal statement about what the system should do. Properties serve as the bridge between human-readable specifications and machine-verifiable correctness guarantees.*


### Property 1: Serialization round-trip preserves all fields

*For any* TelemetryData instance, serializing it to JSON and then deserializing back to TelemetryData should produce an equivalent instance with all fields preserved, including None values.

**Validates: Requirements 1.3, 7.1, 7.3**

### Property 2: iRacing telemetry extraction completeness

*For any* iRacing telemetry data received from simetry, converting it to TelemetryData should successfully extract all iRacing-specific fields (vehicle state, inputs, position, timing, flags, GPS, acceleration, orientation, and tire data) such that fields available in the source data are present (not None) in the TelemetryData.

**Validates: Requirements 2.1, 2.2, 2.3, 2.4, 2.5, 2.6, 2.7, 2.8, 2.9**

### Property 3: ACC telemetry extraction completeness

*For any* ACC telemetry data received from simetry, converting it to TelemetryData should successfully extract all ACC-specific fields (vehicle state, inputs, position, timing, flags, orientation, and tire data when available) such that fields available in the source data are present (not None) in the TelemetryData.

**Validates: Requirements 3.1, 3.2, 3.3, 3.4, 3.5, 3.6, 3.7**

### Property 4: Unavailable fields are represented as None

*For any* game source and TelemetryData instance, fields that are not available from that specific game should be None. For example, ACC telemetry should have None for latitude_deg, longitude_deg, and rate fields (pitch_rate_rps, roll_rate_rps, yaw_rate_rps), while iRacing telemetry should have values for these fields when available.

**Validates: Requirements 1.4**

### Property 5: Producers return TelemetryData

*For any* TelemetryProducer implementation (iRacing, ACC, or Mock), calling the telemetry() method should return a Result containing TelemetryData (not a Moment trait object or any other type).

**Validates: Requirements 4.1**

### Property 6: MockProducer converts SerializableTelemetry correctly

*For any* SerializableTelemetry instance, when MockTelemetryProducer loads it and calls telemetry(), the returned TelemetryData should have equivalent field values to the original SerializableTelemetry.

**Validates: Requirements 4.4**

### Property 7: Deserialization handles missing fields gracefully

*For any* JSON string representing TelemetryData with some fields missing, deserializing it should succeed without panicking, and the missing fields should be set to None in the resulting TelemetryData instance.

**Validates: Requirements 7.4**

## Error Handling

### Conversion Errors

When converting from simetry types to TelemetryData, the following error conditions should be handled:

1. **Missing required fields**: If a field that should always be present (like `point_no` or `timestamp_ms`) cannot be determined, the conversion should use a sensible default rather than failing
2. **Invalid field values**: If a field contains an invalid value (e.g., negative speed), the conversion should either clamp the value to a valid range or set it to None
3. **Type conversion failures**: If converting between units fails (e.g., velocity to m/s), the field should be set to None

### Producer Errors

Producers should return `OcypodeError` in the following cases:

1. **Connection failure**: When the producer cannot connect to the game
2. **Data unavailable**: When telemetry data cannot be retrieved from the game
3. **Conversion failure**: When converting simetry data to TelemetryData fails catastrophically (this should be rare)

### Serialization Errors

1. **Serialization failure**: If TelemetryData cannot be serialized to JSON, return an error to the caller
2. **Deserialization failure**: If JSON cannot be deserialized to TelemetryData due to invalid structure (not just missing fields), return an error

## Testing Strategy

### Unit Testing

Unit tests will verify:

1. **Field extraction**: Test that each conversion function (`from_iracing_state`, `from_acc_state`, `from_serializable`) correctly extracts fields from source data
2. **None handling**: Test that unavailable fields are correctly set to None
3. **Type conversions**: Test that unit conversions (e.g., velocity to m/s, RPM conversion) are correct
4. **Serialization**: Test that TelemetryData can be serialized and deserialized
5. **Analyzer integration**: Test that each analyzer works correctly with TelemetryData instead of Moment

### Property-Based Testing

Property-based tests will use the `proptest` crate to verify the correctness properties defined above. The testing framework will be configured to run a minimum of 100 iterations per property.

**Property test generators**:

1. **TelemetryData generator**: Generate random TelemetryData instances with various combinations of Some and None fields
2. **SerializableTelemetry generator**: Generate random SerializableTelemetry instances for testing MockProducer conversion
3. **JSON generator**: Generate JSON strings with missing fields for testing deserialization robustness

**Property test tagging**:

Each property-based test will be tagged with a comment explicitly referencing the correctness property it implements:

```rust
// **Feature: intermediate-telemetry-representation, Property 1: Serialization round-trip preserves all fields**
#[test]
fn prop_serialization_round_trip() {
    // Test implementation
}
```

### Integration Testing

Integration tests will verify:

1. **End-to-end flow**: Test that telemetry flows from producer through collector to analyzers using TelemetryData
2. **File I/O**: Test that telemetry can be saved to a file and loaded back correctly
3. **Analyzer behavior**: Test that all analyzers produce correct annotations when given TelemetryData

### Test Execution Order

1. Implement TelemetryData struct and basic conversion functions
2. Write unit tests for conversion functions
3. Write property-based tests for correctness properties
4. Update producers to return TelemetryData
5. Write unit tests for updated producers
6. Update analyzers to accept TelemetryData
7. Write unit tests for updated analyzers
8. Write integration tests for end-to-end flow
9. Verify all tests pass before removing SerializableTelemetry

## Migration Strategy

### Phase 1: Introduce TelemetryData

1. Create the `TelemetryData` struct with all fields
2. Implement conversion functions (`from_iracing_state`, `from_acc_state`, `from_serializable`)
3. Add unit tests for conversion functions
4. Keep `SerializableTelemetry` for backward compatibility during migration

### Phase 2: Update Producers

1. Change `TelemetryProducer::telemetry()` to return `Result<TelemetryData, OcypodeError>`
2. Update `IRacingTelemetryProducer` to use `TelemetryData::from_iracing_state()`
3. Update `ACCTelemetryProducer` to use `TelemetryData::from_acc_state()`
4. Update `MockTelemetryProducer` to use `TelemetryData::from_serializable()`
5. Add unit tests for updated producers

### Phase 3: Update Analyzers

1. Change `TelemetryAnalyzer::analyze()` to accept `&TelemetryData`
2. Update all analyzer implementations to use TelemetryData fields
3. Update analyzer unit tests to use TelemetryData
4. Verify all analyzers work correctly with the new interface

### Phase 4: Update Collector and Writer

1. Update `collect_telemetry()` to work with TelemetryData
2. Update telemetry writer to serialize TelemetryData
3. Update telemetry loader to deserialize TelemetryData
4. Add integration tests for end-to-end flow

### Phase 5: Cleanup

1. Remove `SerializableTelemetry` struct
2. Remove `MockMoment` struct (no longer needed)
3. Remove unsafe downcasting code from `SerializableTelemetry::from_moment_boxed()`
4. Update documentation and README
5. Verify all tests pass

## Implementation Notes

### Field Naming Conventions

Field names in `TelemetryData` include units in the name for clarity:
- `_rad` for radians
- `_rps` for radians per second
- `_mps` for meters per second
- `_mps2` for meters per second squared
- `_deg` for degrees
- `_m` for meters
- `_s` for seconds
- `_pct` for percentage (0.0 to 1.0)

### Game-Specific Field Availability

Some fields are only available from specific games:

**iRacing only**:
- `latitude_deg`
- `longitude_deg`
- `lap_distance_m` (absolute distance)
- `pitch_rate_rps`
- `roll_rate_rps`
- `yaw_rate_rps`

**ACC only**:
- None (ACC provides a subset of iRacing fields)

**Both games**:
- All other fields (though availability may vary based on car/track)

### Simetry Library Limitations

The simetry 0.2.3 library has some limitations:

1. **Limited base trait**: The `Moment` trait only exposes a subset of available telemetry data
2. **Game-specific access**: To access additional fields, we need to work with concrete types (`SimState`)
3. **No steering in base trait**: Steering angle is not available through the base `Moment` trait

Our design works around these limitations by having each producer handle game-specific extraction directly, rather than trying to work through the trait abstraction.

### Performance Considerations

1. **Allocation**: TelemetryData is a relatively large struct with many Option fields. Consider using `Box<TelemetryData>` if stack allocation becomes an issue
2. **Cloning**: TelemetryData implements Clone for convenience, but avoid unnecessary clones in hot paths
3. **Serialization**: JSON serialization/deserialization has some overhead. For high-frequency telemetry (100Hz+), consider binary formats if performance becomes an issue

## Future Enhancements

### Additional Games

When adding support for new racing simulations:

1. Add a new variant to `GameSource` enum
2. Create a new producer implementation
3. Add a conversion function (e.g., `from_assetto_corsa_state()`)
4. Update property tests to include the new game
5. Document which fields are available from the new game

### Extended Telemetry Data

If new telemetry fields are needed:

1. Add the field to `TelemetryData` struct
2. Update conversion functions to extract the field
3. Update property tests to verify the field is extracted
4. Update analyzers that need the new field
5. Ensure serialization/deserialization still works

### Performance Optimization

If telemetry processing becomes a bottleneck:

1. Consider using a binary serialization format (e.g., bincode) instead of JSON
2. Use `Arc<TelemetryData>` to avoid cloning in multi-threaded scenarios
3. Implement a telemetry data pool to reuse allocations
4. Profile the conversion functions to identify optimization opportunities
