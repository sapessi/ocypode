mod telemetry;
mod ui;
mod writer;

use std::{
    io,
    path::PathBuf,
    sync::mpsc::{self, SendError},
    thread,
};

use clap::{Parser, Subcommand, ValueEnum, arg};
use egui::Vec2;
use snafu::Snafu;
use telemetry::TelemetryOutput;
#[cfg(windows)]
use telemetry::producer::{ACCTelemetryProducer, IRacingTelemetryProducer};
use ui::analysis::TelemetryAnalysisApp;
use ui::live::{HISTORY_SECONDS, LiveTelemetryApp, config::AppConfig};

#[derive(Debug, Clone, Copy, ValueEnum)]
#[allow(clippy::upper_case_acronyms)]
enum GameSource {
    #[value(name = "iracing")]
    IRacing,
    #[value(name = "acc")]
    ACC,
}

#[derive(Debug, Snafu)]
enum OcypodeError {
    // Errors for the iRacing client
    #[snafu(display("Unable to find iRacing session"))]
    NoIRacingFile { source: io::Error },
    #[snafu(display("Timeout waiting for iRacing session"))]
    IRacingConnectionTimeout,

    // Errors for the ACC client
    #[snafu(display("Timeout waiting for ACC session"))]
    #[allow(dead_code)]
    ACCConnectionTimeout,

    // Errors while reading and broadcasting telemetry data
    #[snafu(display("Missing iRacing client, session not initialized"))]
    MissingIRacingSession,
    #[snafu(display("Telemetry point producer error"))]
    TelemetryProducerError { description: String },
    #[snafu(display("Error broadcasting telemetry data point"))]
    TelemetryBroadcastError {
        source: Box<SendError<TelemetryOutput>>,
    },

    // Errors for the telemetry writer
    #[snafu(display("Error writing telemetry file"))]
    WriterError { source: io::Error },

    // Config managaement errors
    #[snafu(display("Could not find application data directory to save config file"))]
    NoConfigDir,
    #[snafu(display("Error writing config file"))]
    ConfigIOError { source: io::Error },
    #[snafu(display("Error serializing config file"))]
    ConfigSerializeError { source: serde_json::Error },

    // UI errors
    #[snafu(display("Invalid telemetry file: {path}"))]
    InvalidTelemetryFile { path: String },
    #[snafu(display("Error loading telemetry file"))]
    TelemetryLoaderError { source: io::Error },
    #[snafu(display(
        "Legacy telemetry file format detected. This file was created with an older version of Ocypode and is not compatible with the current version. Please re-record your session with the current version."
    ))]
    LegacyTelemetryFormat,
}

impl From<SendError<TelemetryOutput>> for OcypodeError {
    fn from(value: SendError<TelemetryOutput>) -> Self {
        OcypodeError::TelemetryBroadcastError {
            source: Box::new(value),
        }
    }
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Live {
        #[arg(short, long, default_value_t = HISTORY_SECONDS)]
        window: usize,

        #[arg(short, long)]
        output: Option<PathBuf>,

        #[arg(short, long, value_enum)]
        game: GameSource,
    },
    Load {
        #[arg(short, long)]
        input: PathBuf,
    },
}

fn live(window_size: usize, output: Option<PathBuf>, game: GameSource) -> Result<(), OcypodeError> {
    #[cfg(not(windows))]
    {
        eprintln!("Error: Live telemetry is only supported on Windows");
        eprintln!("Supported games: iracing, acc");
        return Err(OcypodeError::TelemetryProducerError {
            description: "Live telemetry is only supported on Windows".to_string(),
        });
    }

    #[cfg(windows)]
    {
        let (telemtry_tx, telemetry_rx) = mpsc::channel::<telemetry::TelemetryOutput>();

        // if we need to write an output file we create a new channel and have the telemetry reader send to both the plotting
        // and writer channels
        if let Some(output_file) = output {
            let (telemetry_writer_tx, telemetry_writer_rx) =
                mpsc::channel::<telemetry::TelemetryOutput>();

            thread::spawn(move || {
                // Instantiate the correct producer based on the game parameter
                match game {
                    GameSource::IRacing => {
                        let telemetry_producer = IRacingTelemetryProducer::default();
                        telemetry::collect_telemetry(
                            telemetry_producer,
                            telemtry_tx,
                            Some(telemetry_writer_tx),
                        )
                        .expect("Error while reading telemetry");
                    }
                    GameSource::ACC => {
                        let telemetry_producer = ACCTelemetryProducer::default();
                        telemetry::collect_telemetry(
                            telemetry_producer,
                            telemtry_tx,
                            Some(telemetry_writer_tx),
                        )
                        .expect("Error while reading telemetry");
                    }
                }
            });
            thread::spawn(move || writer::write_telemetry(&output_file, telemetry_writer_rx));
        } else {
            thread::spawn(move || {
                // Instantiate the correct producer based on the game parameter
                match game {
                    GameSource::IRacing => {
                        let telemetry_producer = IRacingTelemetryProducer::default();
                        telemetry::collect_telemetry(telemetry_producer, telemtry_tx, None)
                            .expect("Error while reading telemetry");
                    }
                    GameSource::ACC => {
                        let telemetry_producer = ACCTelemetryProducer::default();
                        telemetry::collect_telemetry(telemetry_producer, telemtry_tx, None)
                            .expect("Error while reading telemetry");
                    }
                }
            });
        }

        let app_config = AppConfig::from_local_file().unwrap_or(AppConfig {
            window_size_s: window_size,
            ..Default::default()
        });
        let telemetry_window_position = app_config.telemetry_window_position.clone();

        let mut native_options = eframe::NativeOptions::default();
        native_options.viewport = native_options
            .viewport
            .with_always_on_top()
            .with_decorations(false)
            .with_transparent(true)
            .with_inner_size(Vec2::new(500., 200.))
            .with_position(telemetry_window_position);

        eframe::run_native(
            "Ocypode",
            native_options,
            Box::new(|cc| {
                Ok(Box::new(LiveTelemetryApp::new(
                    telemetry_rx,
                    app_config,
                    cc,
                )))
            }),
        )
        .expect("could not start app");
    }

    Ok(())
}

fn load(input: &PathBuf) -> Result<(), OcypodeError> {
    if !input.exists() {
        return Err(OcypodeError::InvalidTelemetryFile {
            path: format!("{:?}", input),
        });
    }
    eframe::run_native(
        "Ocypode Telemetry",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(TelemetryAnalysisApp::from_file(input, cc)))),
    )
    .expect("could not start app");
    Ok(())
}

fn main() {
    #[cfg(debug_assertions)]
    colog::init();

    let cli = Args::parse();
    ctrlc::set_handler(move || {
        println!("Exiting...");
        std::process::exit(0);
    })
    .expect("Could not set Ctrl-C handler");
    match &cli.command {
        Commands::Load { input } => {
            load(input).expect("Error while analyzing telemetry file");
        }
        Commands::Live {
            window,
            output,
            game,
        } => live(*window, output.clone(), *game).expect("Error while running live telemetry"),
    };
}
