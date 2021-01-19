//! Serializable messages representing DHT sensor data.

use serde::{Deserialize, Serialize};

/// Compact JSON from the DHT sensor over serial.
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