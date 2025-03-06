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
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle},
    text::{Alignment, Text, TextStyleBuilder},
    Drawable,
};

// Containers
use circular_buffer::CircularBuffer;
use heapless::String;
use itertools::Itertools;

use core::fmt::Write;
use num_traits::{Num, NumCast};

use crate::{Spi0BusMutex, SENSOR_DATA_SIGNAL};

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
    let align_l_base_bottom = TextStyleBuilder::new()
        .alignment(Alignment::Left)
        .baseline(embedded_graphics::text::Baseline::Bottom)
        .build();
    let co2_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_FOREST_GREEN);
    let temp_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_ORANGE_RED);
    let humidity_text_style = MonoTextStyle::new(&FONT_6X10, Rgb565::CSS_AQUAMARINE);

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
        let temp_min = *temp_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.le(b) { a } else { b })
            .unwrap();
        let temp_max = *temp_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.ge(b) { a } else { b })
            .unwrap();
        let humid_min = *humidity_samples
            .iter()
            .reduce(|a: &f32, b: &f32| if a.le(b) { a } else { b })
            .unwrap();
        let humid_max = *humidity_samples
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
            draw_line_graph(
                &co2_samples,
                co2_max.into(),
                co2_min.into(),
                4,
                Rgb565::CSS_DARK_GREEN,
                Some(Rgb565::new(3, 5, 3)),
                &mut framebuf,
            )
            .unwrap();
        }

        if temp_samples.len() >= 2 {
            draw_line_graph(
                &temp_samples,
                temp_max as i32,
                temp_min as i32,
                38,
                Rgb565::CSS_ORANGE,
                Some(Rgb565::new(3, 5, 3)),
                &mut framebuf,
            )
            .unwrap();
        }

        if humidity_samples.len() >= 2 {
            draw_line_graph(
                &humidity_samples,
                humid_max as i32,
                humid_min as i32,
                72,
                Rgb565::CSS_AQUA,
                Some(Rgb565::new(3, 5, 3)),
                &mut framebuf,
            )
            .unwrap();
        }

        // Draw the text to the screen

        Text::with_text_style(
            &co2_text_buf,
            Point::new(5, 34),
            co2_text_style,
            align_l_base_bottom,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &temp_text_buf,
            Point::new(5, 68),
            temp_text_style,
            align_l_base_bottom,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &humidity_text_buf,
            Point::new(5, 102),
            humidity_text_style,
            align_l_base_bottom,
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

    fn draw_line_graph<'a, I, V, D, C>(
        collection: I,
        graph_max: i32,
        graph_min: i32,
        y_start: i32,
        line_colour: C,
        back_colour: Option<C>,
        target: &mut D,
    ) -> Result<(), D::Error>
    where
        I: IntoIterator<Item = &'a V>,
        I::IntoIter: DoubleEndedIterator,
        V: ?Sized + 'a + Num + NumCast + Clone,
        C: RgbColor,
        D: DrawTarget<Color = C>,
    {
        let mut x_pos: i32 = (DISPLAY_WIDTH - 5) as i32;

        match back_colour {
            Some(c) => {
                let style = PrimitiveStyleBuilder::new().fill_color(c).build();

                Rectangle::new(Point::new(4, y_start), Size::new(120, 30))
                    .into_styled(style)
                    .draw(target)?;
            }
            None => {}
        }

        for (a, b) in collection.into_iter().rev().tuple_windows::<(_, _)>() {
            let range: i32 = if (graph_max - graph_min) == 0 {
                1_i32
            } else {
                graph_max - graph_min
            };

            let a_i32: i32 = match NumCast::from(a.clone()) {
                Some(v) => v,
                None => 0,
            };
            let b_i32: i32 = match NumCast::from(b.clone()) {
                Some(v) => v,
                None => 0,
            };

            let a_y_pos: i32 = y_start + (((graph_max - a_i32) * 30) / range);
            let b_y_pos: i32 = y_start + (((graph_max - b_i32) * 30) / range);

            Line::new(Point::new(x_pos, a_y_pos), Point::new(x_pos - 2, b_y_pos))
                .into_styled(PrimitiveStyle::with_stroke(line_colour, 1))
                .draw(target)?;

            x_pos -= 2;
        }

        return Ok(());
    }
}
