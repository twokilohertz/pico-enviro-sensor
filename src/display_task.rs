// System
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{gpio::Output, spi::Config};
use embedded_graphics_framebuf::FrameBuf;
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
    draw_target::DrawTarget,
    mono_font::{ascii::FONT_6X10, MonoTextStyle},
    pixelcolor::Rgb565,
    prelude::{Point, Primitive, RgbColor, Size, WebColors},
    primitives::{Line, PrimitiveStyle, Rectangle},
    text::{Alignment, Text},
    Drawable,
};

// Containers
use circular_buffer::CircularBuffer;
use heapless::String;
use itertools::Itertools;

use crate::{Spi0BusMutex, SENSOR_DATA_SIGNAL};
use core::{
    fmt::Write,
    ops::{Add, Div, Sub},
};

const DISPLAY_WIDTH: usize = 128;
const DISPLAY_HEIGHT: usize = 128;

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

    // Framebuffer, prevents flickering on redraw
    let buf = [Rgb565::BLACK; DISPLAY_WIDTH * DISPLAY_HEIGHT];
    let mut framebuf = FrameBuf::new(buf, DISPLAY_WIDTH, DISPLAY_HEIGHT);

    // Text styles
    let co2_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_DARK_GREEN);
    let temp_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_ORANGE);
    let humidity_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_AQUA);

    // Format string buffers
    let mut co2_text_buf = String::<16>::new();
    let mut temp_text_buf = String::<16>::new();
    let mut humidity_text_buf = String::<16>::new();

    // Ring buffer for storing past measurement data
    let mut co2_samples = CircularBuffer::<60, u16>::new();
    let mut temp_samples = CircularBuffer::<60, f32>::new();
    let mut humidity_samples = CircularBuffer::<60, f32>::new();

    loop {
        // Clear the framebuffer
        framebuf.clear(Rgb565::BLACK).unwrap();

        // Clear contents of text buffers
        co2_text_buf.clear();
        temp_text_buf.clear();
        humidity_text_buf.clear();

        // Wait on sensor data & format into the buffers
        let sensor_data = SENSOR_DATA_SIGNAL.wait().await;
        write!(&mut co2_text_buf, "CO2: {} ppm", sensor_data.co2).unwrap();
        write!(&mut temp_text_buf, "Temp: {:.1} C", sensor_data.temperature).unwrap();
        write!(&mut humidity_text_buf, "RH: {:.1} %", sensor_data.humidity).unwrap();

        // Record samples
        co2_samples.push_back(sensor_data.co2);
        temp_samples.push_back(sensor_data.temperature);
        humidity_samples.push_back(sensor_data.humidity);

        let co2_min = *co2_samples.iter().min().unwrap();
        let co2_max = *co2_samples.iter().max().unwrap();
        let _temp_min = *temp_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.le(b) { a } else { b })
            .unwrap();
        let _temp_max = *temp_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.ge(b) { a } else { b })
            .unwrap();
        let _humid_min = *humidity_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.le(b) { a } else { b })
            .unwrap();
        let _humid_max = *humidity_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.ge(b) { a } else { b })
            .unwrap();

        /*
           Note about drawing positions:
           The embedded-graphics library follows the OpenGL convention of the
           top-left of the image being (0, 0) with X increasing to the right
           and Y increasing downwards.
        */

        // Draw line graphs

        if co2_samples.len() >= 2 {
            // CO2 line graph happens 4 pixels from the top, with 4 pixels either side

            let mut x_pos: i32 = (DISPLAY_WIDTH - 4) as i32;

            for (a, b) in co2_samples.iter().rev().tuple_windows::<(_, _)>() {
                let co2_range = if (co2_max - co2_min) == 0 {
                    1
                } else {
                    co2_max - co2_min
                };

                let a_y_pos: i32 = (4 + (((co2_max - a) * 30) / co2_range)) as i32;
                let b_y_pos: i32 = (4 + (((co2_max - b) * 30) / co2_range)) as i32;

                Line::new(Point::new(x_pos, a_y_pos), Point::new(x_pos - 2, b_y_pos))
                    .into_styled(PrimitiveStyle::with_stroke(Rgb565::CSS_DARK_GREEN, 1))
                    .draw(&mut framebuf)
                    .unwrap();

                x_pos -= 2;
            }
        }

        // Draw the text to the screen
        Text::with_alignment(
            &co2_text_buf,
            Point::new(1, 12),
            co2_text_style,
            Alignment::Left,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_alignment(
            &temp_text_buf,
            Point::new(1, 24),
            temp_text_style,
            Alignment::Left,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_alignment(
            &humidity_text_buf,
            Point::new(1, 36),
            humidity_text_style,
            Alignment::Left,
        )
        .draw(&mut framebuf)
        .unwrap();

        // Draw the entire framebuffer to the display
        let area: Rectangle = Rectangle::new(
            Point::new(0, 0),
            Size::new(DISPLAY_WIDTH as u32, DISPLAY_HEIGHT as u32),
        );
        display.fill_contiguous(&area, framebuf.data).unwrap();
    }
}
