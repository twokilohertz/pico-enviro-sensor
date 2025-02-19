#![no_std]
#![no_main]

use panic_halt as _;

// Alias for our HAL crate
use rp235x_hal as hal;

// Some things we need
use embedded_hal::delay::DelayNs;
use embedded_hal::digital::OutputPin;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

/// Pico 2 W on-board crystal oscillator frequency (AEL 12.0)
const XTAL_FREQ_HZ: u32 = 12_000_000u32;

#[hal::entry]
fn main() -> ! {
    let mut peripherals = hal::pac::Peripherals::take().unwrap();

    let mut watchdog = hal::Watchdog::new(peripherals.WATCHDOG);

    let clocks = hal::clocks::init_clocks_and_plls(
        XTAL_FREQ_HZ,
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

    let mut led_pin = pins.gpio19.into_push_pull_output();
    loop {
        led_pin.set_high().unwrap();
        timer.delay_ms(1000);
        led_pin.set_low().unwrap();
        timer.delay_ms(1000);
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
