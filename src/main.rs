mod errors;
mod setup_assistant;
mod telemetry;
mod track_metadata;
mod ui;
mod writer;

use std::{path::PathBuf, sync::mpsc, thread};

use clap::{Parser, Subcommand, ValueEnum, arg};
use egui::Vec2;
use errors::OcypodeError;
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

// OcypodeError is now defined in errors.rs

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
    Analysis,
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
        println!("Starting telemetry collection for {:?}...", game);
        println!("Waiting for game connection (this may take up to 10 minutes)...");
        println!("Make sure you're in an active session (on track, not in menus)");

        let (telemtry_tx, telemetry_rx) = mpsc::channel::<telemetry::TelemetryOutput>();

        // if we need to write an output file we create a new channel and have the telemetry reader send to both the plotting
        // and writer channels
        if let Some(output_file) = output {
            let (telemetry_writer_tx, telemetry_writer_rx) =
                mpsc::channel::<telemetry::TelemetryOutput>();

            thread::spawn(move || {
                // Instantiate the correct producer based on the game parameter
                let result = match game {
                    GameSource::IRacing => {
                        let telemetry_producer = IRacingTelemetryProducer::default();
                        telemetry::collect_telemetry(
                            telemetry_producer,
                            telemtry_tx,
                            Some(telemetry_writer_tx),
                        )
                    }
                    GameSource::ACC => {
                        let telemetry_producer = ACCTelemetryProducer::default();
                        telemetry::collect_telemetry(
                            telemetry_producer,
                            telemtry_tx,
                            Some(telemetry_writer_tx),
                        )
                    }
                };

                if let Err(e) = result {
                    // Only log the error if it's not a SendError (which happens when UI closes)
                    match e {
                        OcypodeError::TelemetryBroadcastError { .. } => {
                            // UI closed, this is expected - exit gracefully
                        }
                        _ => {
                            eprintln!("Error while reading telemetry: {:?}", e);
                        }
                    }
                }
            });
            thread::spawn(move || writer::write_telemetry(&output_file, telemetry_writer_rx));
        } else {
            thread::spawn(move || {
                // Instantiate the correct producer based on the game parameter
                let result = match game {
                    GameSource::IRacing => {
                        let telemetry_producer = IRacingTelemetryProducer::default();
                        telemetry::collect_telemetry(telemetry_producer, telemtry_tx, None)
                    }
                    GameSource::ACC => {
                        let telemetry_producer = ACCTelemetryProducer::default();
                        telemetry::collect_telemetry(telemetry_producer, telemtry_tx, None)
                    }
                };

                if let Err(e) = result {
                    // Only log the error if it's not a SendError (which happens when UI closes)
                    match e {
                        OcypodeError::TelemetryBroadcastError { .. } => {
                            // UI closed, this is expected - exit gracefully
                        }
                        _ => {
                            eprintln!("Error while reading telemetry: {:?}", e);
                        }
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

fn analysis() -> Result<(), OcypodeError> {
    eframe::run_native(
        "Ocypode - Telemetry Analysis",
        eframe::NativeOptions::default(),
        Box::new(|cc| {
            Ok(Box::new(TelemetryAnalysisApp::new(cc)))
        }),
    )
    .expect("could not start app");
    Ok(())
}

fn main() {
    // Always initialize logging, not just in debug mode
    colog::init();

    let cli = Args::parse();
    ctrlc::set_handler(move || {
        println!("Exiting...");
        std::process::exit(0);
    })
    .expect("Could not set Ctrl-C handler");
    match &cli.command {
        Commands::Analysis => {
            analysis().expect("Error while analyzing telemetry");
        }
        Commands::Live {
            window,
            output,
            game,
        } => live(*window, output.clone(), *game).expect("Error while running live telemetry"),
    };
}
