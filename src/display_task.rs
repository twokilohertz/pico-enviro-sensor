// System
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{gpio::Output, spi::Config};
use rtt_target::rprintln;

// Display
use display_interface_spi::SPIInterface;
use ssd1351::{
    builder::Builder,
    mode::GraphicsMode,
    properties::{DisplayRotation, DisplaySize},
};

// Graphics
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{Point, WebColors},
    text::{Alignment, Text},
    Drawable,
};

use crate::{Spi0BusMutex, SENSOR_DATA_SIGNAL};
use core::fmt::Write;

/// Output to the SSD1351 display
#[embassy_executor::task]
pub async fn display_output_task(
    spi_bus: &'static Spi0BusMutex,
    cs: Output<'static>,
    dc: Output<'static>,
    rst: &'static mut Output<'static>,
    spi_config: Config,
) {
    rprintln!("Display output task started");

    let spi_dev = SpiDeviceWithConfig::new(spi_bus, cs, spi_config);
    let interface = SPIInterface::new(spi_dev, dc);

    let mut display: GraphicsMode<_> = Builder::new()
        .with_size(DisplaySize::Display128x128)
        .with_rotation(DisplayRotation::Rotate0)
        .connect_interface(interface)
        .into();

    display.reset(rst, &mut embassy_time::Delay).unwrap();
    display.init().unwrap();

    let co2_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_DARK_GREEN);
    let temp_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_ORANGE);
    let humidity_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_AQUA);

    let mut co2_text_buf = heapless::String::<16>::new();
    let mut temp_text_buf = heapless::String::<16>::new();
    let mut humidity_text_buf = heapless::String::<16>::new();

    loop {
        // Clear contents of buffers
        co2_text_buf.clear();
        temp_text_buf.clear();
        humidity_text_buf.clear();

        // Wait on sensor data & format into the buffers
        let data = SENSOR_DATA_SIGNAL.wait().await;
        write!(&mut co2_text_buf, "CO2: {} ppm", data.co2).unwrap();
        write!(&mut temp_text_buf, "Temp: {:.1} C", data.temperature).unwrap();
        write!(&mut humidity_text_buf, "RH: {:.1} %", data.humidity).unwrap();

        // Clear the display
        display.clear();

        // Draw the text to the screen
        Text::with_alignment(
            &co2_text_buf,
            Point::new(1, 12),
            co2_text_style,
            Alignment::Left,
        )
        .draw(&mut display)
        .unwrap();

        Text::with_alignment(
            &temp_text_buf,
            Point::new(1, 24),
            temp_text_style,
            Alignment::Left,
        )
        .draw(&mut display)
        .unwrap();

        Text::with_alignment(
            &humidity_text_buf,
            Point::new(1, 36),
            humidity_text_style,
            Alignment::Left,
        )
        .draw(&mut display)
        .unwrap();
    }
}
