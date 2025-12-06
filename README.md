# Ocypode

I like Rust and I like sim racing. These two passions meet in this little project to read racing simulation telemetry, display it live, and provide helpful alerts to improve driving skills in real-time. In case you are wondering, [Ocypodes are the fastest crabs](https://en.wikipedia.org/wiki/Ocypode).

![Live telemetry with alerts screenshot](/screenshots/mazda_slip.png)

## Supported Games

Ocypode supports telemetry from multiple racing simulations:

- **iRacing** - Full support for all telemetry features
- **Assetto Corsa Competizione (ACC)** - Full support for all telemetry features

Additional games can be added through the [simetry](https://github.com/adnanademovic/simetry) library.

## Why Ocypode
There are lots of telemetry overlays out there. However, I couldn't find one that **(1) gave you a real-time, intuitive view of your driving errors, and (2) didn't require some sort of paid subscription.**

### Real-time alerts 
Traditional telemetry tools require that you save telemetry data and then dive deep to find out what you did wrong and when. Analyzing telemetry data is time-consuming and requires a lot of expertise.

Ocypode analyzes telemetry data in real-time to show intuitive alerts for excessive braking force, wheelspin, missed shifts, scrubbing, etc. This allows you to improve your skills while you drive, without having to dig into the data or switch context.

Ocypode can also save and visualize telemetry data showing the driving alerts it generated.

![Load saved telemetry with alerts](/screenshots/telemetry_analysis_basic.png)

### Free and open source
I want Ocypode to remain a free, open-source tool

## Usage

### Prerequisites

1. Install the [Rust toolchain using `rustup`](https://rustup.rs/)
2. Have one of the supported racing simulations installed and running

### Running Ocypode

Ocypode requires you to specify which racing simulation to connect to using the `--game` (or `-g`) parameter.

#### Live Telemetry Mode

To run Ocypode with live telemetry from iRacing:

```sh
$ cargo run -- live --game iracing
```

To run Ocypode with live telemetry from Assetto Corsa Competizione:

```sh
$ cargo run -- live --game acc
```

#### Saving Telemetry Data

To save telemetry data to a file for later analysis:

```sh
$ cargo run -- live --game iracing --output my_session.jsonl
```

#### Loading Saved Telemetry

To load and analyze previously saved telemetry:

```sh
$ cargo run -- load --input my_session.jsonl
```

### Command-Line Options

**Live Mode:**
```
cargo run -- live [OPTIONS]

Options:
  -g, --game <GAME>        Racing simulation to connect to [possible values: iracing, acc]
  -w, --window <WINDOW>    History window size in seconds [default: 10]
  -o, --output <OUTPUT>    Optional file path to save telemetry data
  -h, --help              Print help
```

**Load Mode:**
```
cargo run -- load [OPTIONS]

Options:
  -i, --input <INPUT>     Path to telemetry file to load
  -h, --help             Print help
```

### Connection Behavior

When you start Ocypode in live mode:

1. The application will attempt to connect to the specified game
2. If the game is not running, Ocypode will wait and retry connection attempts
3. Once connected, telemetry data will be displayed in real-time
4. The application will continue running until you close it (Ctrl+C)

## Migration Notes

### Breaking Changes in v0.3.0

**Important:** Version 0.3.0 introduces breaking changes to the telemetry file format. Telemetry files created with older versions of Ocypode (v0.2.0 and earlier) are **not compatible** with v0.3.0 and later.

#### What Changed

1. **Unified telemetry representation:** The application now uses a single `TelemetryData` struct throughout, replacing the previous `SerializableTelemetry` format
2. **Explicit unit suffixes:** All field names now include explicit unit suffixes for clarity (e.g., `steering_angle_rad`, `speed_mps`, `latitude_deg`)
3. **Improved field naming:** Field names have been updated for consistency and clarity:
   - `steering` → `steering_angle_rad`
   - `lap_distance` → `lap_distance_m`
   - `abs_active` → `is_abs_active`
   - `lat`/`lon` → `latitude_deg`/`longitude_deg`
   - `lat_accel`/`lon_accel` → `lateral_accel_mps2`/`longitudinal_accel_mps2`
   - All orientation fields now include `_rad` suffix
   - All rate fields now include `_rps` suffix
4. **Eliminated unsafe code:** The telemetry system no longer uses unsafe downcasting, improving reliability and maintainability

#### Migration Path

There is **no automatic migration** from older formats. To use your telemetry data with v0.3.0:

1. Re-record your telemetry sessions using the updated application
2. The new format will automatically use the updated field names with explicit units

#### Loading Legacy Files

If you attempt to load a telemetry file created with an older version, you will see this error:

```
Error: This telemetry file was created with an older version of Ocypode and is not 
compatible with the current version. Please re-record your session.
```

For detailed information about the new file format, see [TELEMETRY_FILE_FORMAT.md](TELEMETRY_FILE_FORMAT.md).

For a complete migration guide with code examples, see [MIGRATION.md](MIGRATION.md).

### Previous Breaking Changes

#### v0.2.0

Version 0.2.0 introduced multi-game support and the `SerializableTelemetry` format with a `game_source` field. Files from v0.1.0 are not compatible with v0.2.0 or later.

## Status
The real-time view with basic telemetry and alerts is working. The offline analysis portion is lower priority for a first release. I have created [a project](https://github.com/users/sapessi/projects/1/views/1) to track the first official release.

## Telemetry File Format

Ocypode saves telemetry data in JSON Lines format (`.jsonl`). Each line contains either:
- A telemetry data point with vehicle state and analyzer annotations
- A session change event with track and session metadata

The `TelemetryData` structure uses explicit unit suffixes in field names for clarity:
- `_rad` for radians
- `_rps` for radians per second
- `_mps` for meters per second
- `_mps2` for meters per second squared
- `_deg` for degrees
- `_m` for meters
- `_s` for seconds
- `_pct` for percentage (0.0 to 1.0)

The `game_source` field is always present to identify which racing simulation the data came from. Some fields are game-specific (e.g., GPS coordinates are only available from iRacing).

For complete format specification, see [TELEMETRY_FILE_FORMAT.md](TELEMETRY_FILE_FORMAT.md).

## Development

### Setup

To keep the source code clean, we have a pre-commit git hook that runs the standard `fmt` and `clippy` checks. Before contributing code, run these commands in the repo root:

```sh
$ cargo install rustfmt
$ rustup component add clippy
$ git config --local core.hooksPath .githooks/
```

### Architecture

Ocypode uses the [simetry](https://github.com/adnanademovic/simetry) library as a unified telemetry abstraction layer. This allows the application to support multiple racing simulations through a common interface.

The application uses an intermediate telemetry representation (`TelemetryData`) that decouples analyzers from game-specific implementations. This design:
- Eliminates unsafe code by avoiding type downcasting
- Provides a unified interface for all telemetry data
- Makes it easy to add support for new racing simulations
- Uses explicit unit suffixes in field names for clarity

Key components:
- **TelemetryProducer trait:** Abstracts telemetry data acquisition from different games, converting game-specific data to `TelemetryData`
- **TelemetryAnalyzer trait:** Processes `TelemetryData` to detect driving issues (slip, wheelspin, etc.)
- **TelemetryCollector:** Orchestrates data flow from producers through analyzers to UI and storage
- **TelemetryData struct:** Unified intermediate representation capturing all possible telemetry fields from supported games

### Adding Support for New Games

To add support for a new racing simulation:

1. Ensure the game is supported by the simetry library
2. Add a new variant to the `GameSource` enum in `src/telemetry/mod.rs`
3. Create a conversion function (e.g., `TelemetryData::from_new_game_state()`) that extracts telemetry fields from the game's data structure
4. Create a new producer struct implementing `TelemetryProducer` that uses the conversion function
5. Update the CLI parameter parsing in `main.rs`
6. Document which telemetry fields are available from the new game
7. Test with the new game

The intermediate representation design makes it straightforward to add new games - you only need to implement the conversion logic once, and all analyzers will automatically work with the new game.

### Running Tests

```sh
$ cargo test
```

For property-based tests:

```sh
$ cargo test --features proptest
```