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
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, WebColors},
    primitives::{Circle, PrimitiveStyle},
    Drawable,
};

use crate::Spi0BusMutex;

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

    Circle::new(Point::new(0, 0), 128)
        .into_styled(PrimitiveStyle::with_fill(Rgb565::CSS_CHARTREUSE))
        .draw(&mut display)
        .unwrap();
}
