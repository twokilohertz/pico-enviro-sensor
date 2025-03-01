// System
use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_time::Timer;
use rtt_target::rprintln;

// Sensor
use scd4x::Scd4x;

use crate::{I2c0BusMutex, SENSOR_DATA_SIGNAL};

/// Read CO2/temp./humidity data from the sensor
#[embassy_executor::task]
pub async fn sensor_read_task(i2c_bus: &'static I2c0BusMutex) {
    rprintln!("Sensor read task started");

    Timer::after_millis(30).await; // SCD41 power-up delay
    let i2c_dev = I2cDevice::new(i2c_bus);

    let mut scd41 = Scd4x::new(i2c_dev, embassy_time::Delay);
    scd41.wake_up();
    scd41.reinit().unwrap();

    match scd41.serial_number() {
        Ok(serial) => rprintln!("[SCD41] Serial number: {}", serial),
        Err(error) => rprintln!(
            "[SCD41] Error: did not respond to get_serial_number: {:?}",
            error
        ),
    }

    scd41.start_periodic_measurement().unwrap();

    loop {
        Timer::after_secs(5).await; // start_periodic_measurement returns new sensor data every 5 seconds

        match scd41.measurement() {
            Ok(data) => {
                rprintln!(
                    "[SCD41] CO2: {} ppm, temperature: {} C, humidity: {} RH",
                    data.co2,
                    data.temperature,
                    data.humidity
                );

                SENSOR_DATA_SIGNAL.signal(data);
            }
            Err(error) => {
                rprintln!(
                    "[SCD41] Error: failed to retrieve measurement data: {:?}",
                    error
                );
                break;
            }
        }
    }
}
