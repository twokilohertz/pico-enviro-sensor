#![no_std]
#![no_main]

use rtt_target::{rprintln, rtt_init_print};

use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;

mod sensor_task;

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

/// Entrypoint
#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    // Initialise RTT logging

    rtt_init_print!();
    rprintln!("RTT logging initialised");

    let peripherals = embassy_rp::init(Default::default());

    let sda = peripherals.PIN_4;
    let scl = peripherals.PIN_5;
    let mut i2c_config = embassy_rp::i2c::Config::default(); // TODO: Frequency is 100 MHz by default, should be 400 MHz
    i2c_config.frequency = 400_000u32;
    let i2c_bus = embassy_rp::i2c::I2c::new_blocking(peripherals.I2C0, scl, sda, i2c_config);

    embassy_time::Timer::after_millis(30).await; // SCD41 power-up delay
    let mut scd41 = scd4x::Scd4x::new(i2c_bus, embassy_time::Delay);
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
        embassy_time::Timer::after_secs(5).await;
        match scd41.measurement() {
            Ok(data) => {
                rprintln!(
                    "[SCD41] CO2: {} ppm, temperature: {} C, humidity: {} RH",
                    data.co2,
                    data.temperature,
                    data.humidity
                );
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

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rprintln!("Panicked! {}", info);

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

/// Executable type header for the RP2350 bootloader
#[link_section = ".start_block"]
#[used]
static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

/// Program metadata for picotool
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico Environment Sensor"),
    embassy_rp::binary_info::rp_program_description!(
        c"CO2, temperature & humidity sensing application for the RPi Pico 2 W"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];
