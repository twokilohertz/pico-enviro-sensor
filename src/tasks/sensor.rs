use embassy_embedded_hal::shared_bus::blocking::i2c::I2cDevice;
use embassy_time::{Duration, Instant, Timer};
use rtt_target::debug_rprintln;
use scd4x::Scd4x;

use crate::{
    I2cBus0,
    messages::{Message, MessageBusPublisher},
};

// Every time I come back to this project I have to increase this number and honestly, it sucks
/// Global average atmospheric carbon dioxide concentration (as of last update)
///
/// See: <https://science.nasa.gov/earth/explore/earth-indicators/carbon-dioxide/>
const DEFAULT_BACKGROUND_CO2_PPM: u16 = 429;

/// Sensor power up delay time (datasheet section 2.4.)
const SENSOR_POWER_UP_DELAY: Duration = Duration::from_millis(30);

/// Frequency that the sensor can return new measurement data when in periodic measurement mode (datasheet section 3.6.1.)
const SENSOR_MEASUREMENT_INTERVAL: Duration = Duration::from_secs(5);

/// Reads sensor data & publishes it on the [crate::messages::MessageBus]
#[embassy_executor::task]
pub async fn sensor_read(i2c_bus: &'static I2cBus0, publisher: MessageBusPublisher<'static>) {
    Timer::after(SENSOR_POWER_UP_DELAY).await;

    let device = I2cDevice::new(i2c_bus);

    let mut scd41 = Scd4x::new(device, embassy_time::Delay);
    scd41.wake_up();
    scd41.reinit().unwrap(); // Load settings from EEPROM
    scd41
        .set_automatic_self_calibration_target(DEFAULT_BACKGROUND_CO2_PPM)
        .unwrap();

    debug_rprintln!("Initialised sensor: {:?}", scd41.sensor_variant().unwrap());
    debug_rprintln!(
        "Sensor serial number: {:#x}",
        scd41.serial_number().unwrap()
    );
    debug_rprintln!(
        "Automatic self calibration enabled: {}",
        scd41.automatic_self_calibration().unwrap()
    );
    debug_rprintln!(
        "Automatic self calibration target: {} ppm",
        scd41.automatic_self_calibration_target().unwrap()
    );

    // Main measurement loop

    scd41.start_periodic_measurement().unwrap();

    loop {
        match scd41.measurement() {
            Ok(data) => {
                publisher.publish(Message::SensorMeasurement(data)).await;
            }
            _ => {
                core::hint::cold_path(); // Unlikely

                // Spin in a loop and retry until data is ready, bounded by the measurement interval
                // We assume that the error is here is an I2C NACK: no measurement data available (see datasheet section 3.6.2.)
                // If the error isn't what we expect then we can assume there's bigger problems anyway
                // For future reference: an Err(I2c(I2c(Abort(NoAcknowledge))))

                let start = Instant::now();

                loop {
                    let timed_out = start.elapsed() > SENSOR_MEASUREMENT_INTERVAL;

                    if scd41.data_ready_status().unwrap_or(false) || timed_out {
                        break; // Data's ready!
                    } else {
                        continue; // Keep spinning...
                    }
                }

                continue; // Attempt another measurement read
            }
        }

        Timer::after(SENSOR_MEASUREMENT_INTERVAL).await;
    }
}
