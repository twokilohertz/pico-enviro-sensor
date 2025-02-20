#![no_std]
#![no_main]

use embedded_hal::delay::DelayNs;
use fugit::RateExtU32;
use panic_halt as _; // Needed so its built & linked correctly

// Alias for our HAL crate
use rp235x_hal as hal;

// RP235x HAL
use hal::uart::DataBits;
use hal::uart::StopBits;
use hal::uart::UartConfig;
use hal::uart::UartPeripheral;
use hal::Clock;

mod constants;

#[link_section = ".start_block"]
#[used]
pub static IMAGE_DEF: hal::block::ImageDef = hal::block::ImageDef::secure_exe();

#[hal::entry]
fn main() -> ! {
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

    let uart0_pins = (pins.gpio0.into_function(), pins.gpio1.into_function());

    let uart = UartPeripheral::new(peripherals.UART0, uart0_pins, &mut peripherals.RESETS)
        .enable(
            UartConfig::new(9600_u32.Hz(), DataBits::Eight, None, StopBits::One),
            clocks.peripheral_clock.freq(),
        )
        .unwrap();

    loop {
        uart.write_full_blocking(b"hello, world!\r\n");
        timer.delay_ms(500);
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
