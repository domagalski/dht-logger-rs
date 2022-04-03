use std::error::Error;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use clap::Parser;
use pretty_env_logger;

use dht_logger::{DhtLogger, DhtLoggerConfig};

const LOOP_RETRIES: u32 = 10;

/// Log DHT Sensor readings to various channels.
#[derive(Parser, Debug)]
#[clap(version, name = "dht-logger")]
struct Args {
    /// Config file containing the DHT logging settings
    #[clap(short, long)]
    config: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    let args = Args::parse();
    let config = DhtLoggerConfig::load_yaml(&args.config);

    log::info!("Waiting for serial port: {}", config.port.to_str().unwrap());
    while !config.port.exists() {
        thread::sleep(Duration::from_secs(1));
    }

    let logger = DhtLogger::from_config(&config);
    match logger.port() {
        Some(port) => log::info!("Listening for data on port: {}", port.to_str().unwrap()),
        None => log::info!("Listening for data..."),
    }

    loop {
        logger.read_sensor_and_log_data(LOOP_RETRIES);
    }
}
