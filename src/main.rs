#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use fugit::RateExtU32;
use rtt_target::{rprintln, rtt_init_print};

// RP235x HAL
use rp235x_hal as hal;

// Sensor
use scd4x::Scd4x;

mod constants;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[hal::entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Logging over RTT initialised");

    let mut peripherals = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(peripherals.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        constants::XTAL_FREQ_HZ,
        peripherals.XOSC,
        peripherals.CLOCKS,
        peripherals.PLL_SYS,
        peripherals.PLL_USB,
        &mut peripherals.RESETS,
        &mut watchdog,
    )
    .unwrap();

    let mut timer = hal::Timer::new_timer0(peripherals.TIMER0, &mut peripherals.RESETS, &clocks);

    let sio = hal::Sio::new(peripherals.SIO);

    let pins = hal::gpio::Pins::new(
        peripherals.IO_BANK0,
        peripherals.PADS_BANK0,
        sio.gpio_bank0,
        &mut peripherals.RESETS,
    );

    rprintln!("Core RP2350 hardware initialisation successful");

    // Initialise SCD41 sensor
    let i2c0 = hal::I2C::i2c0(
        peripherals.I2C0,
        pins.gpio4.reconfigure(), // Pin 6 on Pico 2 (SDA)
        pins.gpio5.reconfigure(), // Pin 7 on Pico 2 (SCL)
        400.kHz(),
        &mut peripherals.RESETS,
        &clocks.peripheral_clock,
    );

    timer.delay_ms(30); // Power-up delay
    let mut scd41 = Scd4x::new(i2c0, timer);
    scd41.wake_up();

    match scd41.reinit() {
        Ok(_) => rprintln!("Initialised SCD41"),
        Err(error) => rprintln!("Failed to initialise SCD41: {:?}", error),
    }
    timer.delay_ms(30); // Soft reset delay

    match scd41.serial_number() {
        Ok(serial) => rprintln!("SCD41 serial number: {}", serial),
        Err(error) => rprintln!("SCD41 did not respond to get_serial_number: {:?}", error),
    }

    match scd41.self_test_is_ok() {
        Ok(ok) => {
            if ok {
                rprintln!("SCD41 reported successful self-test")
            } else {
                rprintln!("SCD41 reported unsuccessful self-test!")
            }
        }
        Err(_) => rprintln!("SCD41 failed to perform self-test"),
    }

    match scd41.start_periodic_measurement() {
        Ok(_) => rprintln!("Configured sensor to measure every 5 seconds"),
        Err(error) => rprintln!("SCD41 start_periodic_measurement() failed: {:?}", error),
    }

    loop {
        timer.delay_ms(5010);
        match scd41.measurement() {
            Ok(data) => rprintln!(
                "CO2: {}, temperature: {}, humidity: {}",
                data.co2,
                data.temperature,
                data.humidity
            ),
            Err(error) => rprintln!("SCD41 get_measurement() failed: {:?}", error),
        }
    }

    // loop {
    //     hal::arch::wfi();
    // }
}

#[inline(never)]
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rprintln!("Panicked! {}", info);
    loop {
        hal::arch::nop()
    }
}

/// Program metadata for `picotool info`
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [hal::binary_info::EntryAddr; 3] = [
    hal::binary_info::rp_program_name!(c"Pico Environment Sensor"),
    hal::binary_info::rp_cargo_version!(),
    hal::binary_info::rp_program_build_attribute!(),
];
