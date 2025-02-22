#![no_std]
#![no_main]

// RP235x HAL
use rp235x_hal as hal;

use hal::gpio::FunctionSpi;
use hal::spi::Spi;

// Display
use ssd1351::{
    mode::GraphicsMode,
    prelude::SPIInterface,
    properties::{DisplayRotation, DisplaySize},
};

// Sensor
// use scd4x::Scd4x;

use embedded_graphics::{
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, RgbColor, Size},
    primitives::{PrimitiveStyleBuilder, Rectangle},
    Drawable,
};

use embedded_hal::spi::MODE_0;
use fugit::RateExtU32;
use rtt_target::{rprintln, rtt_init_print};

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

    // Display

    let mosi_pin = pins.gpio19.into_function::<FunctionSpi>();
    let sclk_pin = pins.gpio18.into_function::<FunctionSpi>();
    let cs_pin = pins.gpio17.into_push_pull_output();
    let dc_pin = pins.gpio20.into_push_pull_output();
    let mut rst_pin = pins.gpio21.into_push_pull_output();

    // SPI initialisation
    let spi_pins = (mosi_pin, sclk_pin);
    let spi = Spi::<_, _, _, 8>::new(peripherals.SPI0, spi_pins).init(
        &mut peripherals.RESETS,
        &clocks.peripheral_clock,
        16_u32.MHz(),
        MODE_0,
    );
    let spi_device = embedded_hal_bus::spi::ExclusiveDevice::new_no_delay(spi, cs_pin).unwrap();
    let spi_interface = SPIInterface::new(spi_device, dc_pin);

    let mut display: GraphicsMode<_> = ssd1351::builder::Builder::new()
        .with_size(DisplaySize::Display128x128)
        .with_rotation(DisplayRotation::Rotate0)
        .connect_interface(spi_interface)
        .into();
    display.reset(&mut rst_pin, &mut timer).unwrap();
    display.init().unwrap();

    let rect = Rectangle::new(Point::new(0, 40), Size::new(40, 20)).into_styled(
        PrimitiveStyleBuilder::new()
            .fill_color(Rgb565::CYAN)
            .build(),
    );
    rect.draw(&mut display).unwrap();

    // // Initialise SCD41 sensor
    // let i2c0 = hal::I2C::i2c0(
    //     peripherals.I2C0,
    //     pins.gpio4.reconfigure(), // Pin 6 on Pico 2 (SDA)
    //     pins.gpio5.reconfigure(), // Pin 7 on Pico 2 (SCL)
    //     400.kHz(),
    //     &mut peripherals.RESETS,
    //     &clocks.peripheral_clock,
    // );

    // timer.delay_ms(30); // Power-up delay
    // let mut scd41 = Scd4x::new(i2c0, timer);
    // scd41.wake_up();

    // match scd41.reinit() {
    //     Ok(_) => rprintln!("Initialised SCD41"),
    //     Err(error) => rprintln!("Failed to initialise SCD41: {:?}", error),
    // }
    // timer.delay_ms(30); // Soft reset delay

    // match scd41.serial_number() {
    //     Ok(serial) => rprintln!("SCD41 serial number: {}", serial),
    //     Err(error) => rprintln!("SCD41 did not respond to get_serial_number: {:?}", error),
    // }

    // match scd41.self_test_is_ok() {
    //     Ok(ok) => {
    //         if ok {
    //             rprintln!("SCD41 reported successful self-test")
    //         } else {
    //             rprintln!("SCD41 reported unsuccessful self-test!")
    //         }
    //     }
    //     Err(_) => rprintln!("SCD41 failed to perform self-test"),
    // }

    // match scd41.start_periodic_measurement() {
    //     Ok(_) => rprintln!("Configured sensor to measure every 5 seconds"),
    //     Err(error) => rprintln!("SCD41 start_periodic_measurement() failed: {:?}", error),
    // }

    // loop {
    //     timer.delay_ms(5010);
    //     match scd41.measurement() {
    //         Ok(data) => rprintln!(
    //             "CO2: {}, temperature: {}, humidity: {}",
    //             data.co2,
    //             data.temperature,
    //             data.humidity
    //         ),
    //         Err(error) => rprintln!("SCD41 get_measurement() failed: {:?}", error),
    //     }
    // }

    loop {
        hal::arch::wfi();
    }
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
