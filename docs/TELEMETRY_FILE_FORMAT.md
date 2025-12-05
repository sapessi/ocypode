# Telemetry File Format

## Overview

Ocypode telemetry files use the **JSON Lines** format (also known as newline-delimited JSON or JSONL). Each line in the file is a complete, valid JSON object representing a single telemetry event.

The telemetry data uses the `TelemetryData` intermediate representation, which provides a unified structure for telemetry from all supported racing simulations. This design:
- Captures all possible telemetry fields from supported games
- Uses `Option<T>` types to handle fields that may not be available from all games
- Employs explicit unit suffixes in field names for clarity and type safety
- Eliminates the need for game-specific parsing logic when loading files

## Format Specification

### File Extension
- Recommended: `.jsonl`
- Also supported: `.json` (though this may be misleading as it's not standard JSON)

### Structure

Each line contains a `TelemetryOutput` enum variant, which can be one of two types:

#### 1. DataPoint

Contains telemetry data from a single moment in time.

**Example:**
```json
{"DataPoint":{"point_no":1,"timestamp_ms":1234567890123,"game_source":"IRacing","gear":3,"speed_mps":45.2,"engine_rpm":5500.0,"max_engine_rpm":7200.0,"shift_point_rpm":6800.0,"throttle":0.85,"brake":0.0,"clutch":0.0,"steering_angle_rad":-0.15,"steering_pct":-0.45,"lap_distance_m":1250.5,"lap_distance_pct":0.35,"lap_number":5,"last_lap_time_s":92.456,"best_lap_time_s":91.234,"is_pit_limiter_engaged":false,"is_in_pit_lane":false,"is_abs_active":false,"latitude_deg":36.5844,"longitude_deg":-121.7544,"lateral_accel_mps2":1.2,"longitudinal_accel_mps2":-0.5,"pitch_rad":0.02,"pitch_rate_rps":0.01,"roll_rad":-0.05,"roll_rate_rps":-0.02,"yaw_rad":1.57,"yaw_rate_rps":0.15,"lf_tire_info":{"left_carcass_temp":85.5,"middle_carcass_temp":87.2,"right_carcass_temp":86.1,"left_surface_temp":92.3,"middle_surface_temp":94.1,"right_surface_temp":93.2},"rf_tire_info":{"left_carcass_temp":84.8,"middle_carcass_temp":86.5,"right_carcass_temp":85.3,"left_surface_temp":91.2,"middle_surface_temp":93.4,"right_surface_temp":92.1},"lr_tire_info":{"left_carcass_temp":82.1,"middle_carcass_temp":83.5,"right_carcass_temp":82.8,"left_surface_temp":88.5,"middle_surface_temp":90.2,"right_surface_temp":89.1},"rr_tire_info":{"left_carcass_temp":81.5,"middle_carcass_temp":82.9,"right_carcass_temp":82.2,"left_surface_temp":87.8,"middle_surface_temp":89.5,"right_surface_temp":88.4},"annotations":[]}}
```

**Fields:**

All fields use explicit unit suffixes for clarity:
- `_rad` for radians
- `_rps` for radians per second
- `_mps` for meters per second
- `_mps2` for meters per second squared
- `_deg` for degrees
- `_m` for meters
- `_s` for seconds
- `_pct` for percentage (0.0 to 1.0)

| Field | Type | Description |
|-------|------|-------------|
| `point_no` | `usize` | Sequential point number |
| `timestamp_ms` | `u128` | Unix timestamp in milliseconds |
| `game_source` | `GameSource` | Source game ("IRacing" or "ACC") |
| `gear` | `Option<i8>` | Current gear (-1 for reverse, 0 for neutral, 1+ for forward gears) |
| `speed_mps` | `Option<f32>` | Speed in meters per second |
| `engine_rpm` | `Option<f32>` | Engine RPM |
| `max_engine_rpm` | `Option<f32>` | Maximum engine RPM |
| `shift_point_rpm` | `Option<f32>` | Optimal shift point RPM |
| `throttle` | `Option<f32>` | Throttle position (0.0 to 1.0) |
| `brake` | `Option<f32>` | Brake position (0.0 to 1.0) |
| `clutch` | `Option<f32>` | Clutch position (0.0 to 1.0) |
| `steering_angle_rad` | `Option<f32>` | Steering wheel angle in radians |
| `steering_pct` | `Option<f32>` | Steering as percentage of max angle (-1.0 to 1.0) |
| `lap_distance_m` | `Option<f32>` | Distance traveled from start/finish line in meters |
| `lap_distance_pct` | `Option<f32>` | Percentage distance around lap (0.0 to 1.0) |
| `lap_number` | `Option<u32>` | Current lap number |
| `last_lap_time_s` | `Option<f32>` | Last lap time in seconds |
| `best_lap_time_s` | `Option<f32>` | Best lap time in seconds |
| `is_pit_limiter_engaged` | `Option<bool>` | Whether pit limiter is active |
| `is_in_pit_lane` | `Option<bool>` | Whether vehicle is in pit lane |
| `is_abs_active` | `Option<bool>` | Whether ABS is currently active |
| `latitude_deg` | `Option<f32>` | Latitude in decimal degrees (iRacing only) |
| `longitude_deg` | `Option<f32>` | Longitude in decimal degrees (iRacing only) |
| `lateral_accel_mps2` | `Option<f32>` | Lateral acceleration in m/s² |
| `longitudinal_accel_mps2` | `Option<f32>` | Longitudinal acceleration in m/s² |
| `pitch_rad` | `Option<f32>` | Pitch orientation in radians |
| `pitch_rate_rps` | `Option<f32>` | Pitch rate of change in rad/s (iRacing only) |
| `roll_rad` | `Option<f32>` | Roll orientation in radians |
| `roll_rate_rps` | `Option<f32>` | Roll rate of change in rad/s (iRacing only) |
| `yaw_rad` | `Option<f32>` | Yaw orientation in radians |
| `yaw_rate_rps` | `Option<f32>` | Yaw rate of change in rad/s (iRacing only) |
| `lf_tire_info` | `Option<TireInfo>` | Left front tire information |
| `rf_tire_info` | `Option<TireInfo>` | Right front tire information |
| `lr_tire_info` | `Option<TireInfo>` | Left rear tire information |
| `rr_tire_info` | `Option<TireInfo>` | Right rear tire information |
| `annotations` | `Vec<TelemetryAnnotation>` | Analyzer-generated annotations |

**TireInfo Structure:**
```json
{
  "left_carcass_temp": 85.5,
  "middle_carcass_temp": 87.2,
  "right_carcass_temp": 86.1,
  "left_surface_temp": 92.3,
  "middle_surface_temp": 94.1,
  "right_surface_temp": 93.2
}
```

**TelemetryAnnotation Types:**
- `Slip`: Tire slip detection
- `Scrub`: Tire scrubbing detection
- `ShortShifting`: Early gear shift detection
- `TrailbrakeSteering`: Excessive trail braking with steering
- `Wheelspin`: Wheel spin detection

#### 2. SessionChange

Contains session metadata when a new session is detected.

**Example:**
```json
{"SessionChange":{"track_name":"Laguna Seca","track_configuration":"Full Course","max_steering_angle":17.5,"track_length":"3.60 km","game_source":"IRacing","we_series_id":123,"we_session_id":456,"we_season_id":789,"we_sub_session_id":101,"we_league_id":null}}
```

**Fields:**

| Field | Type | Description |
|-------|------|-------------|
| `track_name` | `String` | Name of the track |
| `track_configuration` | `String` | Track configuration/layout |
| `max_steering_angle` | `f32` | Maximum steering angle in degrees |
| `track_length` | `String` | Track length (formatted string) |
| `game_source` | `GameSource` | Source game ("IRacing" or "ACC") |
| `we_series_id` | `Option<i32>` | iRacing series ID (iRacing only) |
| `we_session_id` | `Option<i32>` | iRacing session ID (iRacing only) |
| `we_season_id` | `Option<i32>` | iRacing season ID (iRacing only) |
| `we_sub_session_id` | `Option<i32>` | iRacing sub-session ID (iRacing only) |
| `we_league_id` | `Option<i32>` | iRacing league ID (iRacing only) |

## Game Source Field

The `game_source` field is **always present** in both `DataPoint` and `SessionChange` variants. This field identifies which racing simulation the telemetry data originated from:

- `"IRacing"`: Data from iRacing
- `"ACC"`: Data from Assetto Corsa Competizione

This allows analysis tools to:
1. Apply game-specific processing logic
2. Handle game-specific data fields appropriately
3. Provide accurate context when displaying telemetry data

## Reading Telemetry Files

### Line-by-Line Processing

Since the file uses JSON Lines format, you should read and parse each line independently:

```rust
use std::fs::File;
use std::io::{BufReader, BufRead};

let file = File::open("telemetry.jsonl")?;
let reader = BufReader::new(file);

for line in reader.lines() {
    let line = line?;
    let output: TelemetryOutput = serde_json::from_str(&line)?;
    
    match output {
        TelemetryOutput::DataPoint(telemetry) => {
            // Process telemetry data
            println!("Point {}: {} from {:?}", 
                telemetry.point_no, 
                telemetry.speed_mps.unwrap_or(0.0),
                telemetry.game_source);
        }
        TelemetryOutput::SessionChange(session) => {
            // Process session change
            println!("New session: {} ({:?})", 
                session.track_name,
                session.game_source);
        }
    }
}
```

## Compatibility Notes

### Breaking Changes from Legacy Format

This format is **not compatible** with older versions of Ocypode that used the `TelemetryPoint` format or the intermediate `SerializableTelemetry` format. Key differences:

1. **Field name changes (from TelemetryPoint):**
   - `point_epoch` → `timestamp_ms`
   - `lap_dist` → `lap_distance_m`
   - `lap_dist_pct` → `lap_distance_pct`
   - `lap_no` → `lap_number`
   - `cur_gear` → `gear` (now `Option<i8>`)
   - `cur_rpm` → `engine_rpm` (now `Option<f32>`)
   - `cur_speed` → `speed_mps` (now `Option<f32>`)

2. **Field name changes (from SerializableTelemetry to TelemetryData):**
   - `steering` → `steering_angle_rad` (clarified units)
   - `lap_distance` → `lap_distance_m` (clarified units)
   - `abs_active` → `is_abs_active` (consistent naming)
   - `lat` → `latitude_deg` (clarified meaning and units)
   - `lon` → `longitude_deg` (clarified meaning and units)
   - `lat_accel` → `lateral_accel_mps2` (clarified meaning and units)
   - `lon_accel` → `longitudinal_accel_mps2` (clarified meaning and units)
   - `pitch` → `pitch_rad` (clarified units)
   - `pitch_rate` → `pitch_rate_rps` (clarified units)
   - `roll` → `roll_rad` (clarified units)
   - `roll_rate` → `roll_rate_rps` (clarified units)
   - `yaw` → `yaw_rad` (clarified units)
   - `yaw_rate` → `yaw_rate_rps` (clarified units)

3. **New required field:**
   - `game_source`: Must be present in all records

4. **Type changes:**
   - Many fields are now `Option<T>` to handle missing data gracefully
   - All fields now use explicit unit suffixes for clarity

### Migration

There is no automatic migration path from the legacy formats. To use telemetry data with the new version:

1. Re-record your telemetry sessions using the updated application
2. The new format will automatically include the `game_source` field and use the updated field names

### Error Detection

When loading a legacy file, the application will detect the incompatible format and display:

```
Error: This telemetry file was created with an older version of Ocypode and is not 
compatible with the current version. Please re-record your session.
```

## Field Naming Conventions

The `TelemetryData` structure uses explicit unit suffixes in all field names to improve clarity and reduce ambiguity:

| Suffix | Meaning | Example |
|--------|---------|---------|
| `_rad` | Radians | `steering_angle_rad`, `pitch_rad` |
| `_rps` | Radians per second | `yaw_rate_rps`, `pitch_rate_rps` |
| `_mps` | Meters per second | `speed_mps` |
| `_mps2` | Meters per second squared | `lateral_accel_mps2` |
| `_deg` | Degrees | `latitude_deg`, `longitude_deg` |
| `_m` | Meters | `lap_distance_m` |
| `_s` | Seconds | `last_lap_time_s` |
| `_pct` | Percentage (0.0 to 1.0) | `lap_distance_pct`, `steering_pct` |

This naming convention makes it immediately clear what units each field uses, reducing the likelihood of unit conversion errors in analysis code.

## Best Practices

1. **File naming:** Use descriptive names with the `.jsonl` extension
   - Good: `laguna_seca_iracing_2024-01-15.jsonl`
   - Avoid: `telemetry.json`

2. **Storage:** Telemetry files can be large. Consider:
   - Compressing old files (gzip works well with JSON Lines)
   - Archiving completed sessions
   - Implementing file rotation for long-running captures

3. **Parsing:** Always handle `Option` fields gracefully:
   ```rust
   let speed = telemetry.speed_mps.unwrap_or(0.0);
   ```

4. **Game-specific processing:** Check `game_source` before accessing game-specific fields:
   ```rust
   if session.game_source == GameSource::IRacing {
       if let Some(series_id) = session.we_series_id {
           // Process iRacing-specific data
       }
   }
   ```

5. **Unit awareness:** Pay attention to field suffixes when performing calculations:
   ```rust
   // Correct: using the right field with explicit units
   let speed_kmh = telemetry.speed_mps.unwrap_or(0.0) * 3.6;
   
   // The field name makes it clear we're working with meters per second
   let lateral_g = telemetry.lateral_accel_mps2.unwrap_or(0.0) / 9.81;
   ```

## Game-Specific Field Availability

Some telemetry fields are only available from specific racing simulations:

**iRacing-specific fields:**
- `latitude_deg` - GPS latitude coordinate
- `longitude_deg` - GPS longitude coordinate
- `lap_distance_m` - Absolute distance from start/finish line
- `pitch_rate_rps` - Pitch rate of change
- `roll_rate_rps` - Roll rate of change
- `yaw_rate_rps` - Yaw rate of change

**ACC-specific limitations:**
- GPS coordinates are not available (set to `null`)
- Absolute lap distance is not available (set to `null`)
- Rate data (pitch/roll/yaw rates) are not available (set to `null`)

**Note:** Due to limitations in the simetry 0.2.3 library, some iRacing-specific fields may not be populated even though the game provides them. Future versions may expose additional fields through direct game API access.

## Example File

A typical telemetry file might look like:

```jsonl
{"SessionChange":{"track_name":"Laguna Seca","track_configuration":"Full Course","max_steering_angle":17.5,"track_length":"3.60 km","game_source":"IRacing","we_series_id":123,"we_session_id":456,"we_season_id":789,"we_sub_session_id":101,"we_league_id":null}}
{"DataPoint":{"point_no":0,"timestamp_ms":1234567890000,"game_source":"IRacing","gear":1,"speed_mps":15.2,...,"annotations":[]}}
{"DataPoint":{"point_no":1,"timestamp_ms":1234567890016,"game_source":"IRacing","gear":2,"speed_mps":25.8,...,"annotations":[]}}
{"DataPoint":{"point_no":2,"timestamp_ms":1234567890032,"game_source":"IRacing","gear":2,"speed_mps":35.1,...,"annotations":[{"Wheelspin":{"avg_rpm_increase_per_gear":{"2":150.0},"cur_gear":2,"cur_rpm_increase":280.5,"is_wheelspin":true}}]}}
...
```

## Version History

- **v0.3.0** (Current): Unified `TelemetryData` format with explicit unit suffixes, improved field naming
- **v0.2.0** (Legacy): Multi-game support with `game_source` field, `SerializableTelemetry` format (incompatible)
- **v0.1.0** (Legacy): iRacing-only with `TelemetryPoint` format (incompatible)
