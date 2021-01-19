use std::error::Error;
use std::path::Path;

use clap::{App, Arg};
use pretty_env_logger;

use dht_logger::DhtLogger;

const LOOP_RETRIES: u32 = 10;

fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let matches = App::new("DHT Sensor Logger")
        .about("Log DHT Sensor readings to various channels.")
        .arg(
            Arg::with_name("config")
                .short("c")
                .long("config")
                .value_name("CONFIG")
                .help("Config file containing DHT logging settings.")
                .takes_value(true)
                .required(true),
        )
        .get_matches();

    let config_path = Path::new(matches.value_of("config").unwrap());
    let logger = DhtLogger::from_config(config_path);
    match logger.port() {
        Some(name) => log::info!("Listening for data on port: {}", name),
        None => log::info!("Listening for data..."),
    }

    loop {
        logger.read_sensor_and_log_data(LOOP_RETRIES);
    }
}
