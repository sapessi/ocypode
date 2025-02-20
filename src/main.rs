mod live;
mod telemetry;
mod writer;

use std::{
    io,
    path::PathBuf,
    sync::mpsc::{self, SendError},
    thread,
};

use clap::{arg, Parser, Subcommand};
use egui::Vec2;
use live::LiveTelemetryApp;
use snafu::Snafu;
use telemetry::{producer::IRacingTelemetryProducer, TelemetryPoint};

const HISTORY_SECONDS: usize = 5;

#[derive(Debug, Snafu)]
enum OcypodeError {
    #[snafu(display("Unable to find iRacing session"))]
    NoIRacingFile { source: io::Error },
    #[snafu(display("Timeout waiting for iRacing session"))]
    IRacingConnectionTimeout,
    #[snafu(display("Telemetry point producer error"))]
    TelemetryProducerError { description: &'static str },
    #[snafu(display("Error broadcasting telemetry data point"))]
    TelemetryBroadcastError { source: SendError<TelemetryPoint> },
    #[snafu(display("Error writing telemetry file"))]
    WriterError { source: io::Error },
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
    },
    Load {
        #[arg(short, long)]
        input: Option<PathBuf>,
    },
}

fn live(window_size: usize, output: Option<PathBuf>) -> Result<(), OcypodeError> {
    let (telemtry_tx, telemetry_rx) = mpsc::channel::<telemetry::TelemetryPoint>();

    // if we need to write an output file we create a new channel and have the telemetry reader send to both the plotting
    // and writer channels
    if let Some(output_file) = output {
        let (telemetry_writer_tx, telemetry_writer_rx) =
            mpsc::channel::<telemetry::TelemetryPoint>();
        thread::spawn(move || {
            let telemetry_producer = IRacingTelemetryProducer::default();
            telemetry::collect_telemetry(
                telemetry_producer,
                telemtry_tx,
                Some(telemetry_writer_tx),
            )
            .expect("Error while reading telemetry");
        });
        thread::spawn(move || writer::write_telemetry(&output_file, telemetry_writer_rx));
    } else {
        thread::spawn(move || {
            let telemetry_producer = IRacingTelemetryProducer::default();
            telemetry::collect_telemetry(telemetry_producer, telemtry_tx, None)
                .expect("Error while reading telemetry");
        });
    }

    let app = LiveTelemetryApp::new(telemetry_rx, window_size);
    let mut native_options = eframe::NativeOptions::default();
    native_options.viewport = native_options
        .viewport
        .with_always_on_top()
        .with_decorations(false)
        .with_transparent(true)
        .with_inner_size(Vec2::new(500., 200.));
    eframe::run_native(
        "Monitor app",
        native_options,
        Box::new(|_| Ok(Box::new(app))),
    )
    .expect("could not start app");
    Ok(())
}

fn main() {
    let cli = Args::parse();
    ctrlc::set_handler(move || {
        println!("Exiting...");
        std::process::exit(0);
    })
    .expect("Could not set Ctrl-C handler");
    match &cli.command {
        Commands::Load { input: _ } => {
            todo!()
        }
        Commands::Live { window, output } => {
            live(*window, output.clone()).expect("Error while running live telemetry")
        }
    };
}
