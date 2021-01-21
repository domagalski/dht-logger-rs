//! DHT Logger
//!
//! This crate is for logging measurement from a device reading DHT sensors and writing the
//! measurements over a serial connection. The hardware producing the data does not matter, but it
//! must be logging data over serial in JSON with fields for temperature, humidity, and heat index.
//! Here's a pretty version of an example reading:
//! ```json
//! {
//!   "sensor_label": {
//!     "t": 20.0,
//!     "h": 50.0,
//!     "hi": 20.0
//!   },
//!   "another_sensor": {
//!     "error": "some error message"
//!   }
//! }
//! ```
//!
//! This code has been tested using
//! [arduino-dht-logger](https://github.com/domagalski/arduino-dht-logger) as the hardware source
//! providing data over serial.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Error, ErrorKind};
use std::net::{SocketAddrV4, UdpSocket};
use std::path::Path;
use std::thread;
use std::time::Duration;

use chrono::Utc;
use log;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use serde_yaml;
use serialport::{self, SerialPort};

pub mod messages;
use messages::*;
pub use messages::{Measurement, SensorData};

#[cfg(test)]
pub mod tests;

/// Contain results with `std::io::Error` as the `Error` implementation.
pub type Result<T> = std::result::Result<T, Error>;

const BUFFER_SIZE: usize = 1024;
const TIMEOUT: Duration = Duration::from_secs(4);

/// Configuration of a DHT Logger client.
///
/// Example configuration YAML:
/// ```yaml
/// # Serial port configuration
/// port: /dev/ttyUSB0
/// baud: 115200
///
/// # Configure how the sensor data is logged.
/// logger_config:
///   # verbose: true tells the logger to
///   # use log::info! for sensor readings
///   verbose: true
/// ```
#[derive(Debug, Deserialize, Serialize)]
pub struct DhtLoggerConfig {
    port: String,
    baud: u32,
    logger_config: HashMap<String, Value>,
}

/// DHT Logger client.
///
/// This is for reading data over serial and logging it using various means.
///
/// Supported logging methods:
/// * `verbose`: Log incoming data using `log::info!`
pub struct DhtLogger {
    port: RefCell<Box<dyn SerialPort>>,
    verbose: bool,
    udp_addrs: Vec<SocketAddrV4>,
    udp_socket: Option<UdpSocket>,
}

impl DhtLogger {
    /// Create a DHT logger from an existing serial port.
    ///
    /// Args:
    /// * `port`: An interface to use as a serial port.
    /// * `logger_config`: Configure how data is logged. See the `DhtLoggerConfig` documentation.
    pub fn new(port: Box<dyn SerialPort>, logger_config: HashMap<String, Value>) -> DhtLogger {
        let verbose = if let Some(verbose) = logger_config.get("verbose") {
            if let Value::Bool(verbose) = verbose {
                *verbose
            } else {
                panic!("logger.verbose must be boolean, got value: {}", verbose)
            }
        } else {
            false
        };

        let default = Value::Array(Vec::new());
        let udp_addrs: Vec<SocketAddrV4> = logger_config
            .get("udp")
            .unwrap_or(&default)
            .as_array()
            .expect("logger.udp must be a list")
            .iter()
            .map(|addr| {
                addr.as_str().expect(&format!(
                    "UDP addresses must be strings, got value: {}",
                    addr
                ))
            })
            .map(|addr| {
                addr.parse()
                    .expect(&format!("Failed to parse IP:PORT, got value: {}", addr))
            })
            .collect();

        let udp_socket = match udp_addrs.len() {
            0 => None,
            _ => Some(UdpSocket::bind("0.0.0.0:0").unwrap()),
        };

        DhtLogger {
            port: RefCell::new(port),
            verbose,
            udp_addrs,
            udp_socket,
        }
    }

    /// Create a DHT logger from loading a YAML configuration file.
    pub fn from_config(config_file: &Path) -> DhtLogger {
        // Panic if the config file doesn't exist or can't be parsed.
        let config_file = File::open(config_file).unwrap();
        let DhtLoggerConfig {
            port,
            baud,
            logger_config,
        } = match serde_yaml::from_reader(config_file) {
            Ok(dht_logger) => dht_logger,
            Err(_) => panic!("YAML parse error in DHT logger config."),
        };

        let port = serialport::new(port.clone(), baud)
            .timeout(TIMEOUT)
            .open()
            .expect(&format!("Failed to open port: {}", port));

        // trace log serial port parameters
        log::trace!("Data bits: {:?}", port.data_bits());
        log::trace!("Flow control: {:?}", port.flow_control());
        log::trace!("Parity: {:?}", port.parity());
        log::trace!("Stop bits: {:?}", port.stop_bits());
        log::trace!("Timeout: {:?}", port.timeout());

        DhtLogger::new(port, logger_config)
    }

    /// Get the name of the serial port.
    pub fn port(&self) -> Option<String> {
        self.port.borrow().name()
    }

    /// Read sensor data over serial and return it. This blocks until data is readable over the
    /// serial interface or a timeout occurs.
    pub fn read_sensor(&self) -> Result<DhtSensors> {
        let mut buffer: [u8; BUFFER_SIZE] = [0; BUFFER_SIZE];
        let n_bytes = self.port.borrow_mut().read(&mut buffer)?;
        let timestamp = Utc::now();
        let raw = match serde_json::from_slice::<Value>(&buffer[..n_bytes])? {
            Value::Object(map) => map,
            _ => {
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    "DHT logger data must be a JSON mapping",
                ))
            }
        };

        let mut sensors = HashMap::new();
        for (key, value) in raw.iter() {
            let value = if let Value::Object(map) = value {
                map
            } else {
                panic!("Sensor value must be a JSON mapping, got value: {}", value);
            };

            let measurement = if let Some(error) = value.get("e") {
                let error = if let Value::String(error) = error {
                    error
                } else {
                    panic!("Error value must be a string, got value: {}", error);
                };
                Measurement::new(None, Some(error))
            } else {
                let raw: DhtDataRaw = serde_json::from_value(Value::Object(value.clone()))?;
                Measurement::new(Some(SensorData::from(raw)), None)
            };

            if let Some(error) = measurement.get_error() {
                log::warn!("Error reading '{}' sensor: {}", key, error);
                continue;
            }

            let data = measurement.get_data().unwrap();
            sensors.insert(String::from(key), data);
        }

        Ok(DhtSensors {
            timestamp,
            data: sensors,
        })
    }

    /// Wait for the sensor to return data for a specified amount of retries. If the number of
    /// attempts to read data exceed the allowed number of retries, the last error message is
    /// returned. If an error occurs, this function sleeps for 100s. All sensor read errors are
    /// logged to `log::trace!` as they arrive.
    pub fn wait_for_sensor(&self, retries: u32) -> Result<DhtSensors> {
        let mut retry: u32 = 0;
        loop {
            match self.read_sensor() {
                Ok(measurement) => {
                    return Ok(measurement);
                }
                Err(err) => {
                    retry += 1;
                    log::trace!("{}", err);
                    if retry == retries {
                        return Err(err);
                    }
                    thread::sleep(Duration::from_millis(100));
                }
            }
        }
    }

    /// Log a measurement to the all of the logging channels
    /// configured in the logger config for the DHT Logger.
    pub fn log_measurement(&self, measurement: DhtSensors) -> Result<()> {
        // Verbose logging
        let data_pretty = serde_json::to_string_pretty(&measurement)?;
        let data_pretty = format!("Received measurement:\n{}", data_pretty);
        if self.verbose {
            log::info!("{}", data_pretty);
        } else {
            log::debug!("{}", data_pretty);
        }

        // UDP logging
        if let Some(udp_socket) = &self.udp_socket {
            let data_json = serde_json::to_vec(&DhtSensorsSerde::from(measurement))?;
            log::trace!("{}", std::str::from_utf8(data_json.as_slice()).unwrap());
            for addr in self.udp_addrs.iter() {
                let bytes_sent = udp_socket.send_to(data_json.as_slice(), addr)?;
                log::trace!("Sent {} bytes to UDP addr: {:?}", bytes_sent, addr);
            }
        }

        Ok(())
    }

    /// Read data from the DHT sensor serial interface and log data to all logging channels.
    ///
    /// Args:
    /// * `retries`: Number of sensor read retries (see `wait_for_sensor docs) before giving up.
    pub fn read_sensor_and_log_data(&self, retries: u32) {
        let measurement = match self.wait_for_sensor(retries) {
            Ok(data) => data,
            Err(_) => return,
        };

        if let Err(err) = self.log_measurement(measurement) {
            log::warn!("{}", err);
        }
    }
}
