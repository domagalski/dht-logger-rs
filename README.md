# dht-logger
Read DHT temperature sensor data formatted in JSON over a serial
interface and log it.

This crate is for logging measurement from a device reading DHT sensors and
writing the measurements over a serial connection. The hardware producing the
data does not matter, but it must be logging data over serial in JSON with
fields for temperature, humidity, and heat index. Here's a pretty version of an
example reading:
```json
{
  "sensor_label": {
    "t": 20.0,
    "h": 50.0,
    "hi": 20.0
  },
  "another sensor": {
    "error": "some error message"
  }
}
```

This code has been tested using
[arduino-dht-logger](https://github.com/domagalski/arduino-dht-logger) as the
hardware source providing data over serial.

## Example

The following example creates a DHT logger from a configuration file, then
reads data from the serial port and logs it to whatever logging channels are
configured.

```rust
use std::path::Path;
use dht_logger::DhtLogger;

let config_path = Path::new("example_config.yaml");
let logger = DhtLogger::from_config(config_path);
logger.read_sensor_and_log_data(10);
```
