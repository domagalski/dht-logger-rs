use std::collections::HashMap;
use std::io::{prelude::*, Error, ErrorKind};
use std::net::UdpSocket;
use std::ptr;
use std::time::Duration;

use portpicker;
use serde_json::Value;
use serialport::{ClearBuffer, DataBits, FlowControl, Parity, SerialPort, StopBits};

use super::*;

// Validate that sensor data can be read
#[test]
fn test_read_sensor() {
    let port = Box::new(MockSerialPort::new(1));
    let sensor_config = HashMap::new();
    let logger_config = HashMap::new();

    let logger = DhtLogger::new(port, sensor_config, logger_config);
    assert!(logger.read_sensor().is_ok());
    assert!(logger.wait_for_sensor(10).is_ok());
}

// Validate that read errors are detected
#[test]
fn test_empty_sensor() {
    let port = Box::new(MockSerialPort::new(0));
    let sensor_config = HashMap::new();
    let logger_config = HashMap::new();

    let logger = DhtLogger::new(port, sensor_config, logger_config);
    assert!(logger.read_sensor().is_err());
}

// Validate that data logged over UDP shows up
#[test]
fn test_udp_logger() {
    // Create random UDP addr
    let udp_port = portpicker::pick_unused_port().expect("no ports available");
    let udp_addr = format!("127.0.0.1:{}", udp_port);
    let udp_sock = UdpSocket::bind(udp_addr.clone())
        .expect(&format!("failed to bind to udp address: {}", udp_addr));
    udp_sock
        .set_read_timeout(Some(Duration::from_secs(1)))
        .expect("failed to set read timeout");

    // Add random UDP addr to logger_config
    let mut logger_config = HashMap::new();
    logger_config.insert(
        String::from("udp"),
        Value::Array(vec![Value::String(udp_addr.clone())]),
    );

    // Create mock serial port
    let data_size = 10;
    let port = Box::new(MockSerialPort::new(data_size));
    let sensor_config = HashMap::new();

    // Send fake data over UDP
    let logger = DhtLogger::new(port, sensor_config, logger_config);
    logger.read_sensor_and_log_data(10);

    // Deserialize data over UDP
    let mut buffer: [u8; super::BUFFER_SIZE] = [0; super::BUFFER_SIZE];
    let (n_bytes, _) = udp_sock
        .recv_from(&mut buffer)
        .expect("Failed to read data from socket");
    let data = serde_json::from_slice::<DhtSensorsSerde>(&buffer[..n_bytes])
        .expect("failed to deserialize");
    let data = DhtSensors::from_serde(data);
    assert!(data.is_ok());
    let data = data.unwrap();
    assert_eq!(data.data.len(), data_size);

    // Validate that the data is equal to what it should be equal to.
    for i in 0..data_size {
        let value = 1.0 * (i as f32);
        let data = data.data.get(&format!("{}", i)).unwrap();
        assert_eq!(data.temperature, value);
        assert_eq!(data.humidity, value);
        assert_eq!(data.heat_index, value);
    }
}

//////////////////
// TEST HELPERS //
//////////////////

type SerialResult<T> = std::result::Result<T, serialport::Error>;
type RawSensors = HashMap<String, DhtDataRaw>;

#[derive(Clone)]
struct MockSerialPort {
    data: RawSensors,
}

impl MockSerialPort {
    fn new(length: usize) -> MockSerialPort {
        let mut data = RawSensors::new();
        for i in 0..length {
            let value = 1.0 * (i as f32);
            data.insert(
                format!("{}", i),
                DhtDataRaw {
                    t: value,
                    h: value,
                    hi: value,
                },
            );
        }

        MockSerialPort { data }
    }
}

impl Write for MockSerialPort {
    fn write(&mut self, buffer: &[u8]) -> Result<usize> {
        Ok(buffer.len())
    }

    fn flush(&mut self) -> Result<()> {
        Ok(())
    }
}

impl Read for MockSerialPort {
    fn read(&mut self, buffer: &mut [u8]) -> Result<usize> {
        let serialized = serde_json::to_vec(&self.data).unwrap();

        // data = 2 is the default for an empty json
        if serialized.len() <= 2 {
            return Err(Error::new(ErrorKind::UnexpectedEof, "no data to be read"));
        } else if serialized.len() > buffer.len() {
            return Err(Error::new(ErrorKind::InvalidData, "too much data"));
        }

        let s_ptr = serialized.as_ptr();
        let b_ptr = buffer.as_mut_ptr();
        unsafe {
            ptr::copy(s_ptr, b_ptr, serialized.len());
        }

        Ok(serialized.len())
    }
}

impl SerialPort for MockSerialPort {
    fn name(&self) -> Option<String> {
        None
    }

    fn baud_rate(&self) -> SerialResult<u32> {
        Ok(0)
    }

    fn data_bits(&self) -> SerialResult<DataBits> {
        Ok(DataBits::Eight)
    }

    fn flow_control(&self) -> SerialResult<FlowControl> {
        Ok(FlowControl::None)
    }

    fn parity(&self) -> SerialResult<Parity> {
        Ok(Parity::None)
    }

    fn stop_bits(&self) -> SerialResult<StopBits> {
        Ok(StopBits::One)
    }

    fn timeout(&self) -> Duration {
        Duration::from_nanos(0)
    }

    fn set_baud_rate(&mut self, _: u32) -> SerialResult<()> {
        Ok(())
    }

    fn set_data_bits(&mut self, _: DataBits) -> SerialResult<()> {
        Ok(())
    }

    fn set_flow_control(&mut self, _: FlowControl) -> SerialResult<()> {
        Ok(())
    }

    fn set_parity(&mut self, _: Parity) -> SerialResult<()> {
        Ok(())
    }

    fn set_stop_bits(&mut self, _: StopBits) -> SerialResult<()> {
        Ok(())
    }

    fn set_timeout(&mut self, _: Duration) -> SerialResult<()> {
        Ok(())
    }

    fn write_request_to_send(&mut self, _: bool) -> SerialResult<()> {
        Ok(())
    }

    fn write_data_terminal_ready(&mut self, _: bool) -> SerialResult<()> {
        Ok(())
    }

    fn read_clear_to_send(&mut self) -> SerialResult<bool> {
        Ok(true)
    }

    fn read_data_set_ready(&mut self) -> SerialResult<bool> {
        Ok(true)
    }

    fn read_ring_indicator(&mut self) -> SerialResult<bool> {
        Ok(true)
    }

    fn read_carrier_detect(&mut self) -> SerialResult<bool> {
        Ok(true)
    }

    fn bytes_to_read(&self) -> SerialResult<u32> {
        Ok(0)
    }

    fn bytes_to_write(&self) -> SerialResult<u32> {
        Ok(0)
    }

    fn clear(&self, _: ClearBuffer) -> SerialResult<()> {
        Ok(())
    }

    fn try_clone(&self) -> SerialResult<Box<(dyn SerialPort + 'static)>> {
        Ok(Box::new(self.clone()))
    }

    fn set_break(&self) -> SerialResult<()> {
        Ok(())
    }

    fn clear_break(&self) -> SerialResult<()> {
        Ok(())
    }
}
