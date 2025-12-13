# Analyzer Configuration Reference

## Overview

This document provides detailed information about the telemetry analyzers used in the Setup Assistant, including their detection thresholds, configuration parameters, and implementation details.

## Analyzer Architecture

All analyzers implement the `TelemetryAnalyzer` trait:

```rust
pub trait TelemetryAnalyzer {
    fn analyze(
        &mut self,
        telemetry: &TelemetryData,
        session_info: &SessionInfo,
    ) -> Vec<TelemetryAnnotation>;
}
```

Analyzers process telemetry data points and return annotations when they detect issues. These annotations are then aggregated by the Setup Assistant into findings.

## Entry Oversteer Analyzer

**Purpose**: Detects when the rear slides out during braking and turn-in.

**File**: `src/telemetry/entry_oversteer_analyzer.rs`

### Configuration Constants

```rust
const MIN_BRAKE_PCT: f32 = 0.3;           // 30% brake application required
const MIN_STEERING_PCT: f32 = 0.1;        // 10% steering input required
const OVERSTEER_THRESHOLD: f32 = 1.5;     // Yaw rate must exceed expected by 1.5x
```

### State Management

- **Window Size**: 10 samples (configurable via generic parameter)
- **Minimum Points**: 5 samples required before detection begins
- **Moving Average**: Uses `SumTreeSMA` for efficient yaw-to-steering ratio calculation

### Detection Logic

1. **Phase Detection**: Only analyzes during corner entry (brake > 30% AND steering > 10%)
2. **Baseline Calculation**: Builds a moving average of expected yaw rate response to steering input
3. **Oversteer Detection**: Triggers when actual yaw rate exceeds expected by 1.5x
4. **Annotation**: Creates `EntryOversteer` annotation with expected and actual yaw rates

### Telemetry Requirements

- `brake`: Brake pedal position (0.0 to 1.0)
- `steering_pct`: Steering input as percentage (-1.0 to 1.0)
- `yaw_rate_rps`: Yaw rate in radians per second

### Tuning Guidance

**Increase sensitivity** (detect more oversteer):
- Lower `MIN_BRAKE_PCT` (e.g., 0.2) - detect at lighter braking
- Lower `MIN_STEERING_PCT` (e.g., 0.05) - detect with less steering
- Lower `OVERSTEER_THRESHOLD` (e.g., 1.3) - trigger with smaller yaw rate excess

**Decrease sensitivity** (detect less oversteer):
- Raise `MIN_BRAKE_PCT` (e.g., 0.4) - only detect during heavy braking
- Raise `MIN_STEERING_PCT` (e.g., 0.15) - require more steering input
- Raise `OVERSTEER_THRESHOLD` (e.g., 1.7) - require larger yaw rate excess

## Mid-Corner Analyzer

**Purpose**: Detects understeer or oversteer during the apex/coasting phase.

**File**: `src/telemetry/mid_corner_analyzer.rs`

### Configuration Constants

```rust
const MAX_COASTING_THROTTLE: f32 = 0.15;  // 15% max throttle for coasting
const MAX_COASTING_BRAKE: f32 = 0.15;     // 15% max brake for coasting
const MIN_STEERING_PCT: f32 = 0.1;        // 10% steering input required
const UNDERSTEER_SPEED_LOSS_THRESHOLD: f32 = 0.5;  // 0.5 m/s speed loss
const OVERSTEER_THRESHOLD: f32 = 1.5;     // Yaw rate must exceed expected by 1.5x
```

### State Management

- **Window Size**: 10 samples (configurable via generic parameter)
- **Minimum Points**: 5 samples required before oversteer detection
- **Previous Speed**: Tracks speed from previous telemetry point for understeer detection
- **Moving Average**: Uses `SumTreeSMA` for yaw-to-steering baseline

### Detection Logic

**Understeer Detection**:
1. **Phase Detection**: Coasting (throttle < 15% AND brake < 15% AND steering > 10%)
2. **Speed Loss**: Compares current speed to previous speed
3. **Trigger**: Speed loss > 0.5 m/s indicates understeer
4. **Annotation**: Creates `MidCornerUndersteer` with speed loss value

**Oversteer Detection**:
1. **Phase Detection**: Same coasting phase as understeer
2. **Baseline Calculation**: Builds expected yaw rate response
3. **Trigger**: Actual yaw rate exceeds expected by 1.5x
4. **Annotation**: Creates `MidCornerOversteer` with yaw rate excess

### Telemetry Requirements

- `throttle`: Throttle pedal position (0.0 to 1.0)
- `brake`: Brake pedal position (0.0 to 1.0)
- `steering_pct`: Steering input as percentage (-1.0 to 1.0)
- `speed_mps`: Vehicle speed in meters per second
- `yaw_rate_rps`: Yaw rate in radians per second

### Tuning Guidance

**Coasting Phase Detection**:
- Adjust `MAX_COASTING_THROTTLE` and `MAX_COASTING_BRAKE` to define what counts as "coasting"
- Lower values = stricter definition of coasting
- Higher values = more lenient, catches more mid-corner issues

**Understeer Sensitivity**:
- Lower `UNDERSTEER_SPEED_LOSS_THRESHOLD` (e.g., 0.3) - detect smaller speed losses
- Higher threshold (e.g., 0.7) - only detect significant understeer

**Oversteer Sensitivity**:
- Same as Entry Oversteer Analyzer's `OVERSTEER_THRESHOLD`

## Brake Lock Analyzer

**Purpose**: Detects brake locking through ABS activation patterns.

**File**: `src/telemetry/brake_lock_analyzer.rs`

### Configuration Constants

```rust
const MIN_BRAKE_PCT: f32 = 0.3;  // 30% brake application required
```

### State Management

- **ABS Activation Count**: Tracks number of ABS activations in current braking zone
- **In Braking Zone**: Boolean flag for braking zone detection
- **Previous Brake**: Tracks brake input from previous telemetry point

### Detection Logic

1. **Braking Zone Detection**: 
   - Entry: brake crosses above 30% threshold
   - Exit: brake drops below 30% threshold
   - Count resets on zone exit

2. **ABS Detection**: Monitors `is_abs_active` flag during braking zones

3. **Classification**: 
   - Currently creates general brake lock annotation
   - Future: Will classify as front/rear based on tire slip data when available

4. **Annotation**: Creates `FrontBrakeLock` or `RearBrakeLock` with activation count

### Telemetry Requirements

- `brake`: Brake pedal position (0.0 to 1.0)
- `is_abs_active`: Boolean indicating ABS activation
- `tire_info` (optional): For front/rear classification (not yet implemented)

### Tuning Guidance

**Braking Zone Detection**:
- Lower `MIN_BRAKE_PCT` (e.g., 0.2) - detect lighter braking zones
- Higher threshold (e.g., 0.4) - only detect heavy braking

**Note**: This analyzer's sensitivity is primarily determined by the game's ABS system, not by configurable thresholds.

## Tire Temperature Analyzer

**Purpose**: Monitors tire temperatures over time to detect overheating or cold tires.

**File**: `src/telemetry/tire_temperature_analyzer.rs`

### Configuration Constants

```rust
const OPTIMAL_TEMP_MIN: f32 = 80.0;       // 80°C minimum optimal temperature
const OPTIMAL_TEMP_MAX: f32 = 95.0;       // 95°C maximum optimal temperature
const HISTORY_DURATION_S: usize = 60;     // 60 second history window
const MIN_SAMPLES: usize = 10;            // 10 samples before detection
const SAMPLE_RATE_HZ: f32 = 60.0;         // Assumed telemetry rate
```

### State Management

- **Temperature History**: `VecDeque` of temperature snapshots
- **Sample Counter**: Tracks telemetry points for sampling interval
- **Sample Interval**: Samples every 60 telemetry points (1 per second at 60Hz)

### Detection Logic

1. **Temperature Calculation**: Averages all 12 tire surface temperatures (3 per tire × 4 tires)

2. **Sampling**: Samples once per second to avoid excessive memory usage

3. **History Management**: Maintains 60-second rolling window of samples

4. **Overheating Detection**:
   - Requires at least 10 samples
   - Calculates average temperature over history window
   - Triggers if average > 95°C

5. **Cold Tire Detection**:
   - Requires at least 10 samples
   - Calculates average temperature over history window
   - Triggers if average < 80°C

6. **Annotations**: Creates `TireOverheating` or `TireCold` with average temperature

### Telemetry Requirements

- `lf_tire_info`: Left front tire information
- `rf_tire_info`: Right front tire information
- `lr_tire_info`: Left rear tire information
- `rr_tire_info`: Right rear tire information

Each `TireInfo` contains:
- `left_surface_temp`: Temperature of left third of tire
- `middle_surface_temp`: Temperature of middle third of tire
- `right_surface_temp`: Temperature of right third of tire

### Tuning Guidance

**Optimal Temperature Range**:
- Adjust `OPTIMAL_TEMP_MIN` and `OPTIMAL_TEMP_MAX` for different tire compounds
- GT3 tires: 80-95°C (default)
- Softer compounds: May run cooler (75-90°C)
- Harder compounds: May run hotter (85-100°C)

**Detection Sensitivity**:
- Increase `HISTORY_DURATION_S` (e.g., 90) - require longer sustained temperature
- Decrease `HISTORY_DURATION_S` (e.g., 30) - detect shorter temperature issues
- Increase `MIN_SAMPLES` (e.g., 15) - require more data before detection
- Decrease `MIN_SAMPLES` (e.g., 5) - detect issues faster

**Sampling Rate**:
- Adjust `SAMPLE_RATE_HZ` if your telemetry runs at different frequency
- Higher rate = more frequent sampling = more memory usage
- Lower rate = less frequent sampling = less responsive detection

## Bottoming Out Analyzer

**Purpose**: Detects suspension bottoming through pitch changes and speed loss.

**File**: `src/telemetry/bottoming_out_analyzer.rs`

### Configuration Constants

```rust
const MIN_PITCH_CHANGE_RAD: f32 = 0.05;   // 0.05 radians minimum pitch change
const MAX_STEERING_PCT: f32 = 0.2;        // 20% maximum steering (filters for straights)
const MIN_SPEED_LOSS_MPS: f32 = 0.5;      // 0.5 m/s minimum speed loss
```

### State Management

- **Previous Pitch**: Tracks pitch angle from previous telemetry point
- **Previous Speed**: Tracks speed from previous telemetry point

### Detection Logic

1. **Phase Filtering**: Only analyzes when steering < 20% (straights or over bumps)

2. **Pitch Change Detection**: Calculates absolute pitch change from previous point

3. **Speed Loss Detection**: Calculates speed loss from previous point

4. **Trigger Conditions**:
   - Pitch change > 0.05 radians AND
   - Speed loss > 0.5 m/s AND
   - Steering < 20%

5. **Annotation**: Creates `BottomingOut` with pitch change and speed loss values

### Telemetry Requirements

- `pitch_rad`: Vehicle pitch angle in radians
- `speed_mps`: Vehicle speed in meters per second
- `steering_pct`: Steering input as percentage (-1.0 to 1.0)

### Tuning Guidance

**Pitch Sensitivity**:
- Lower `MIN_PITCH_CHANGE_RAD` (e.g., 0.03) - detect smaller pitch changes
- Higher threshold (e.g., 0.07) - only detect severe bottoming

**Speed Loss Sensitivity**:
- Lower `MIN_SPEED_LOSS_MPS` (e.g., 0.3) - detect with less speed loss
- Higher threshold (e.g., 0.7) - require more significant speed loss

**Steering Filter**:
- Lower `MAX_STEERING_PCT` (e.g., 0.1) - stricter straight-line filter
- Higher threshold (e.g., 0.3) - allow more steering input

## Existing Analyzers

The Setup Assistant also uses these existing analyzers:

### Slip Analyzer

**Purpose**: Detects front tire slip (understeer).

**File**: `src/telemetry/slip_analyzer.rs`

**Context-Based Classification**:
- Slip during braking → Corner Entry Understeer
- Slip during throttle → Corner Exit Understeer
- Slip during coasting with speed loss → Mid-Corner Understeer

### Scrub Analyzer

**Purpose**: Detects front tire scrubbing during corner entry.

**File**: `src/telemetry/scrub_analyzer.rs`

**Classification**: Always maps to Corner Entry Understeer

### Wheelspin Analyzer

**Purpose**: Detects rear wheelspin during acceleration.

**File**: `src/telemetry/wheelspin_analyzer.rs`

**Classification**: Always maps to Corner Exit Power Oversteer

### Trailbrake Steering Analyzer

**Purpose**: Detects excessive trail braking into corners.

**File**: `src/telemetry/trailbrake_steering_analyzer.rs`

**Classification**: Maps to Excessive Trailbraking

### Short Shifting Analyzer

**Purpose**: Detects when the driver shifts gears before reaching the optimal RPM.

**File**: `src/telemetry/short_shifting_analyzer.rs`

**Configuration Constants**:
```rust
const DEFAULT_SHORT_SHIFT_SENSITIVITY: f32 = 100.0;  // RPM tolerance below optimal
```

**Game-Specific Behavior**:
- **iRacing**: Uses game-provided `shift_point_rpm` when available
- **ACC**: `shift_point_rpm` is automatically populated during telemetry conversion as 87% of `max_engine_rpm` since ACC doesn't provide shift point data through the simetry API

**Detection Logic**:
1. Tracks previous gear and RPM
2. Detects gear upshifts (current gear > previous gear)
3. Compares shift RPM to `shift_point_rpm` from telemetry data
4. Triggers if shift occurred more than 100 RPM below optimal

**Telemetry Requirements**:
- `gear`: Current gear number
- `engine_rpm`: Current engine RPM
- `shift_point_rpm`: Optimal shift point (provided by iRacing, estimated for ACC)

**Classification**: Not mapped to setup issues (shifting technique, not setup)

## Performance Considerations

### Analyzer Performance

All analyzers are designed to run at high frequency (60+ Hz):

- **Lightweight Logic**: Minimal computation per telemetry point
- **Efficient Data Structures**: `VecDeque` for rolling windows, `SumTreeSMA` for moving averages
- **No Allocations in Hot Path**: Reuse existing buffers where possible

### Memory Usage

**Per Analyzer**:
- Entry Oversteer: ~40 bytes (10-sample window)
- Mid-Corner: ~44 bytes (10-sample window + previous speed)
- Brake Lock: ~12 bytes (counters and flags)
- Tire Temperature: ~1.5 KB (60-second history at 1 Hz)
- Bottoming Out: ~8 bytes (previous pitch and speed)

**Total**: ~1.6 KB for all analyzers combined

### CPU Usage

At 60 Hz telemetry rate:
- Each analyzer processes in < 1 microsecond
- Total analyzer overhead: < 10 microseconds per telemetry point
- Negligible impact on overall application performance

## Testing

### Unit Tests

Each analyzer includes comprehensive unit tests:
- Normal operation scenarios
- Edge cases (missing data, extreme values)
- State management (session resets)
- Threshold boundary conditions

### Property-Based Tests

Critical analyzers include property-based tests using `proptest`:
- Entry Oversteer: Validates detection across random valid inputs
- Mid-Corner: Validates understeer detection with varied parameters
- Brake Lock: Validates ABS detection with random brake levels
- Tire Temperature: Validates overheating detection with varied temperatures
- Bottoming Out: Validates detection with random pitch/speed combinations

### Integration Tests

The Setup Assistant includes integration tests that verify:
- Annotation extraction and categorization
- Finding aggregation with occurrence counting
- Corner phase classification
- Slip classification by context

## Future Enhancements

### Planned Improvements

1. **Adaptive Thresholds**: Learn optimal thresholds based on car and track
2. **Machine Learning**: Use ML to improve detection accuracy
3. **Track-Specific Tuning**: Adjust thresholds based on track characteristics
4. **User Configuration**: Allow users to adjust sensitivity via UI
5. **Tire Slip Classification**: Implement front/rear brake lock classification using tire slip data

### Extensibility

Adding new analyzers is straightforward:

1. Create new analyzer struct implementing `TelemetryAnalyzer`
2. Add new `TelemetryAnnotation` variant
3. Add new `FindingType` variant
4. Update `annotation_to_finding_type` mapping
5. Add recommendations to `RecommendationEngine`
6. Write unit and property-based tests

No changes needed to UI or core Setup Assistant logic.

## References

- **ACC Setup Guide**: Basis for recommendation mappings
- **Telemetry File Format**: See `docs/TELEMETRY_FILE_FORMAT.md`
- **Setup Assistant User Guide**: See `docs/SETUP_ASSISTANT.md`
- **Performance Documentation**: See `docs/PERFORMANCE.md`
