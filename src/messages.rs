//! Serializable messages representing DHT sensor data.

use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Result;

/// Serde JSON from the DHT sensor over serial.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DhtDataRaw {
    pub t: f32,
    pub h: f32,
    pub hi: f32,
}

/// A single reading for a DHT sensor.
#[derive(Copy, Clone, Debug, PartialEq, Deserialize, Serialize)]
pub struct SensorData {
    pub temperature: f32,
    pub humidity: f32,
    pub heat_index: f32,
}

/// Convert the RAW Json to SensorData so it can be re-serialized with full field names.
impl From<DhtDataRaw> for SensorData {
    fn from(data: DhtDataRaw) -> Self {
        SensorData {
            temperature: data.t,
            humidity: data.h,
            heat_index: data.hi,
        }
    }
}

/// Container of measurements from all DHT sensors in one reading.
///
/// The JSON serialization is not compact. For smaller JSON messages, use `DhtSensorsSerde`.
#[derive(Debug, Deserialize, Serialize)]
pub struct DhtSensors {
    pub timestamp: DateTime<Utc>,
    pub data: HashMap<String, SensorData>,
}

impl DhtSensors {
    /// Decode a `DntSensorsSerde` struct into DhtSensors.
    ///
    /// If not all hashmaps in DhtSensorsPacked have
    pub fn from_serde(data: DhtSensorsSerde) -> Result<DhtSensors> {
        let mut lengths = HashSet::new();
        lengths.insert(data.o.len());
        lengths.insert(data.t.len());
        lengths.insert(data.h.len());
        lengths.insert(data.hi.len());

        if lengths.len() != 1 {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "length mismatch in serde data",
            ));
        }

        let mut sensor_data = HashMap::new();
        for (i, key) in data.o.iter().enumerate() {
            sensor_data.insert(
                key.clone(),
                SensorData {
                    temperature: data.t[i],
                    humidity: data.h[i],
                    heat_index: data.hi[i],
                },
            );
        }

        Ok(DhtSensors {
            timestamp: data.ts,
            data: sensor_data,
        })
    }
}

/// A more compactly serialized verson of DhtSensors for serializing via JSON
///
/// This is not intended on being human-readable. For human-readability, use `DhtSensors` instead.
#[derive(Debug, Deserialize, Serialize)]
pub struct DhtSensorsSerde {
    pub ts: DateTime<Utc>,
    pub o: Vec<String>,
    pub t: Vec<f32>,
    pub h: Vec<f32>,
    pub hi: Vec<f32>,
}

impl From<DhtSensors> for DhtSensorsSerde {
    fn from(data: DhtSensors) -> DhtSensorsSerde {
        let timestamp = data.timestamp;
        let mut order = Vec::new();
        let mut temperature = Vec::new();
        let mut humidity = Vec::new();
        let mut heat_index = Vec::new();

        for (key, value) in data.data.iter() {
            order.push(key.clone());
            temperature.push(value.temperature);
            humidity.push(value.humidity);
            heat_index.push(value.heat_index);
        }

        DhtSensorsSerde {
            ts: timestamp,
            o: order,
            t: temperature,
            h: humidity,
            hi: heat_index,
        }
    }
}

union DhtDataUnion<'a> {
    error: &'a str,
    data: SensorData,
}

/// Data container for a DHT sensor measurement that contains either an error or data.
/// ```
/// use dht_logger::{Measurement, SensorData};
/// // Example test data
/// let error = "test";
/// let data = SensorData {
///     temperature: 0.0,
///     humidity: 0.0,
///     heat_index: 0.0,
/// };
///
/// // Create a measurement containing an error
/// let measurement = Measurement::new(None, Some(error));
/// assert!(measurement.get_data().is_none());
/// assert!(measurement.get_error().is_some());
/// assert_eq!(measurement.get_error().unwrap(), error);
///
/// // Create a measurement containing data
/// let measurement = Measurement::new(Some(data), None);
/// assert!(measurement.get_data().is_some());
/// assert!(measurement.get_error().is_none());
/// assert_eq!(measurement.get_data().unwrap(), data);
/// ```
pub struct Measurement<'a> {
    error: bool,
    data: DhtDataUnion<'a>,
}

impl<'a> Measurement<'a> {
    /// Create a new measurement of a DHT sensor.
    ///
    /// Args:
    /// * `data`: Sensor data from one DHT sensor.
    /// * `error`: Error indicating a failure to read a DHT sensor.
    pub fn new(data: Option<SensorData>, error: Option<&'a str>) -> Measurement {
        if (data.is_some() && error.is_some()) || (data.is_none() && error.is_none()) {
            panic!("Exactly one of data or error must be a Some type.");
        }

        if let Some(data) = data {
            return Measurement {
                error: false,
                data: DhtDataUnion { data },
            };
        }

        if let Some(error) = error {
            return Measurement {
                error: true,
                data: DhtDataUnion { error },
            };
        }

        // This should never happen
        Measurement {
            error: true,
            data: DhtDataUnion {
                error: "initialization error",
            },
        }
    }

    /// Get the data contained by the measurement, if it exists.
    pub fn get_data(&self) -> Option<SensorData> {
        if self.has_data() {
            unsafe { Some(self.data.data) }
        } else {
            None
        }
    }

    /// Get the error contained by the measurement, if it exists.
    pub fn get_error(&self) -> Option<&'a str> {
        if self.has_error() {
            unsafe { Some(self.data.error) }
        } else {
            None
        }
    }

    /// Check if the measurement has data.
    pub fn has_data(&self) -> bool {
        !self.error
    }

    /// Check if the measurement has an error.
    pub fn has_error(&self) -> bool {
        self.error
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    // Test that Measurement panics when giving None twice
    #[test]
    #[should_panic]
    fn test_measurement_new_both_none() {
        Measurement::new(None, None);
    }

    // Test that Measurement panics when giving Some twice
    #[test]
    #[should_panic]
    fn test_measurement_new_both_some() {
        let error = "test";
        let data = SensorData {
            temperature: 0.0,
            humidity: 0.0,
            heat_index: 0.0,
        };
        Measurement::new(Some(data), Some(error));
    }

    // Test that SensorData can be converted from a DhtDataRaw
    #[test]
    fn test_convert_from_raw() {
        let raw = DhtDataRaw {
            t: 21.3,
            h: 52.7,
            hi: 22.8,
        };

        let data = SensorData::from(raw.clone());
        assert_eq!(raw.t, data.temperature);
        assert_eq!(raw.h, data.humidity);
        assert_eq!(raw.hi, data.heat_index);
    }
}
