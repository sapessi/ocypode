# Ocypode

I like Rust and I like sim racing. These two passions meet in this little project to read racing simulation telemetry, display it live, and provide helpful alerts to improve driving skills in real-time. In case you are wondering, [Ocypodes are the fastest crabs](https://en.wikipedia.org/wiki/Ocypode).

![Live telemetry with alerts screenshot](/screenshots/mazda_slip.png)

## Supported Games

Ocypode supports telemetry from multiple racing simulations:

- **iRacing** - Full support for all telemetry features
- **Assetto Corsa Competizione (ACC)** - Full support for all telemetry features

Additional games can be added through the [simetry](https://github.com/adnanademovic/simetry) library.

## Why Ocypode
There are lots of telemetry overlays out there. However, I couldn't find one that **(1) gave you a real-time, intuitive view of your driving errors, (2) provided intelligent setup recommendations, and (3) didn't require some sort of paid subscription.**

### Real-time alerts 
Traditional telemetry tools require that you save telemetry data and then dive deep to find out what you did wrong and when. Analyzing telemetry data is time-consuming and requires a lot of expertise.

Ocypode analyzes telemetry data in real-time to show intuitive alerts for excessive braking force, wheelspin, missed shifts, scrubbing, etc. This allows you to improve your skills while you drive, without having to dig into the data or switch context.

Ocypode can also save and visualize telemetry data showing the driving alerts it generated.

![Load saved telemetry with alerts](/screenshots/telemetry_analysis_basic.png)

### Setup Assistant
![Live setup assistant](/screenshots/live_setup_assistant.png)

The Setup Assistant takes telemetry analysis a step further by automatically detecting handling issues and providing specific car setup recommendations. It monitors your driving in real-time, identifies problems like understeer, oversteer, brake locking, and tire temperature issues, then suggests precise setup changes based on proven methodology.

You confirm the issues you actually feel in the car, and the Setup Assistant provides targeted recommendations organized by category (aero, suspension, brakes, etc.). This bridges the gap between raw telemetry data and actionable setup improvements, helping you optimize your car without needing deep setup expertise.

For detailed information, see the [Setup Assistant User Guide](docs/SETUP_ASSISTANT.md).

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