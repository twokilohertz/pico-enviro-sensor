use core::fmt::Write;

// System
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDeviceWithConfig;
use embassy_rp::{gpio::Output, spi::Config};
use embedded_graphics_framebuf::FrameBuf;
use fixed::types::U16F16;
use rtt_target::debug_rprintln;

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
    mono_font::{ascii::FONT_6X10, MonoTextStyleBuilder},
    pixelcolor::{Rgb565, Rgb888},
    prelude::{Point, Primitive, RgbColor, Size, WebColors},
    primitives::{Line, PrimitiveStyle, Rectangle, StyledDrawable},
    text::{Alignment, Text, TextStyleBuilder},
    Drawable,
};

// Containers
use circular_buffer::CircularBuffer;
use heapless::String;

use crate::{Spi0BusMutex, SENSOR_DATA_SIGNAL};

const DISPLAY_WIDTH: usize = 128;
const DISPLAY_HEIGHT: usize = 128;
const DISPLAY_PADDING: usize = 5;
const LINE_GRAPH_WIDTH: u32 = (DISPLAY_WIDTH - (DISPLAY_PADDING * 2)) as u32;
const LINE_GRAPH_HEIGHT: u32 = 36;
const CO2_TEXT_X: i32 = DISPLAY_PADDING as i32 + 1;
const CO2_TEXT_Y: i32 = DISPLAY_PADDING as i32 + LINE_GRAPH_HEIGHT as i32;
const TEMP_TEXT_X: i32 = CO2_TEXT_X;
const TEMP_TEXT_Y: i32 = (DISPLAY_PADDING as i32 + LINE_GRAPH_HEIGHT as i32) * 2;
const HUMID_TEXT_X: i32 = CO2_TEXT_X;
const HUMID_TEXT_Y: i32 = (DISPLAY_PADDING as i32 + LINE_GRAPH_HEIGHT as i32) * 3;

type SensorDataBuffer = CircularBuffer<60, U16F16>;

/// Output to the SSD1351 display
#[embassy_executor::task]
pub async fn display_output_task(
    spi_bus: &'static Spi0BusMutex,
    cs: Output<'static>,
    dc: Output<'static>,
    rst: &'static mut Output<'static>,
    spi_config: Config,
) {
    debug_rprintln!("Display output task started");

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
    let text_style = TextStyleBuilder::new()
        .alignment(Alignment::Left)
        .baseline(embedded_graphics::text::Baseline::Bottom)
        .build();
    let drop_shadow_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::BLACK)
        .build();
    let co2_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::CSS_FOREST_GREEN)
        .build();
    let temp_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::CSS_ORANGE_RED)
        .build();
    let humidity_style = MonoTextStyleBuilder::new()
        .font(&FONT_6X10)
        .text_color(Rgb565::CSS_DARK_CYAN)
        .build();

    // Format string buffers
    let mut co2_text_buf = String::<20>::new();
    let mut temp_text_buf = String::<20>::new();
    let mut humidity_text_buf = String::<20>::new();

    // Ring buffer for storing past measurement data
    let mut co2_samples = SensorDataBuffer::new();
    let mut temp_samples = SensorDataBuffer::new();
    let mut humidity_samples = SensorDataBuffer::new();

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
        co2_samples.push_back(U16F16::from_num(sensor_data.co2));
        temp_samples.push_back(U16F16::from_num(sensor_data.temperature));
        humidity_samples.push_back(U16F16::from_num(sensor_data.humidity));
        co2_samples.make_contiguous();
        temp_samples.make_contiguous();
        humidity_samples.make_contiguous();

        let co2_min = *co2_samples.iter().min().unwrap();
        let co2_max = *co2_samples.iter().max().unwrap();
        let temp_min = *temp_samples.iter().min().unwrap();
        let temp_max = *temp_samples.iter().max().unwrap();
        let humid_min = *humidity_samples.iter().min().unwrap();
        let humid_max = *humidity_samples.iter().max().unwrap();

        /*
           Note about drawing positions:
           The embedded-graphics library follows the OpenGL convention of the
           top-left of the image being (0, 0) with X increasing to the right
           and Y increasing downwards.
        */

        // Draw line graphs

        draw_line_graph(
            Rectangle::new(
                Point::new(DISPLAY_PADDING as i32, DISPLAY_PADDING as i32),
                Size::new(LINE_GRAPH_WIDTH, LINE_GRAPH_HEIGHT),
            ),
            co2_min,
            co2_max,
            co2_samples.as_slices().0,
            Rgb565::CSS_DARK_GREEN,
            Some(Rgb888::new(24, 24, 24).into()),
            &mut framebuf,
        );

        draw_line_graph(
            Rectangle::new(
                Point::new(
                    DISPLAY_PADDING as i32,
                    (LINE_GRAPH_HEIGHT + (DISPLAY_PADDING as u32 * 2)) as i32,
                ),
                Size::new(LINE_GRAPH_WIDTH, LINE_GRAPH_HEIGHT),
            ),
            temp_min,
            temp_max,
            temp_samples.as_slices().0,
            Rgb565::CSS_ORANGE,
            Some(Rgb888::new(24, 24, 24).into()),
            &mut framebuf,
        );

        draw_line_graph(
            Rectangle::new(
                Point::new(
                    DISPLAY_PADDING as i32,
                    ((LINE_GRAPH_HEIGHT * 2) + (DISPLAY_PADDING as u32 * 3)) as i32,
                ),
                Size::new(LINE_GRAPH_WIDTH, LINE_GRAPH_HEIGHT),
            ),
            humid_min,
            humid_max,
            humidity_samples.as_slices().0,
            Rgb565::CSS_AQUA,
            Some(Rgb888::new(24, 24, 24).into()),
            &mut framebuf,
        );

        // Draw the text to the screen

        Text::with_text_style(
            &co2_text_buf,
            Point::new(CO2_TEXT_X + 1, CO2_TEXT_Y + 1),
            drop_shadow_style,
            text_style,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &co2_text_buf,
            Point::new(CO2_TEXT_X, CO2_TEXT_Y),
            co2_style,
            text_style,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &temp_text_buf,
            Point::new(TEMP_TEXT_X + 1, TEMP_TEXT_Y + 1),
            drop_shadow_style,
            text_style,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &temp_text_buf,
            Point::new(TEMP_TEXT_X, TEMP_TEXT_Y),
            temp_style,
            text_style,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &humidity_text_buf,
            Point::new(HUMID_TEXT_X + 1, HUMID_TEXT_Y + 1),
            drop_shadow_style,
            text_style,
        )
        .draw(&mut framebuf)
        .unwrap();

        Text::with_text_style(
            &humidity_text_buf,
            Point::new(HUMID_TEXT_X, HUMID_TEXT_Y),
            humidity_style,
            text_style,
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

fn draw_line_graph<C, D>(
    bounds: Rectangle,
    y_min: U16F16,
    y_max: U16F16,
    samples: &[U16F16],
    fg_colour: C,
    bg_colour: Option<C>,
    target: &mut D,
) where
    C: RgbColor,
    D: DrawTarget<Color = C>,
{
    // Draw background colour first, if supplied

    if let Some(bg_col) = bg_colour {
        let _ = bounds.draw_styled(&PrimitiveStyle::with_fill(bg_col), target);
    }

    // Draw the data points

    if samples.len() < 2 {
        // Drawing a line requires a minimum of two points
        return;
    }

    let graph_width = U16F16::from_num(bounds.size.width);
    let graph_height = U16F16::from_num(bounds.size.height);
    let y_range = y_max - y_min;
    let mut x_offset: i32 = bounds.top_left.x;
    let n_samples: U16F16 = U16F16::from_num(samples.len());
    let mut n_samples_seen: U16F16 = U16F16::from_num(1);

    for sample in samples.windows(2) {
        let x_pos_start: i32 = x_offset;
        let y_pos_start: i32 = bounds.top_left.y
            + (graph_height - (((sample[0] - y_min) / y_range) * graph_height)).to_num::<i32>();
        let x_pos_end: i32 = bounds.top_left.x
            + (((n_samples_seen + U16F16::from_num(1)) / n_samples)
                * (graph_width - U16F16::from_num(1)))
            .to_num::<i32>();
        let y_pos_end: i32 = bounds.top_left.y
            + (graph_height - (((sample[1] - y_min) / y_range) * graph_height)).to_num::<i32>();

        let _ = Line::new(
            Point::new(x_pos_start, y_pos_start),
            Point::new(x_pos_end, y_pos_end),
        )
        .into_styled(PrimitiveStyle::with_stroke(fg_colour, 1))
        .draw(target);

        x_offset = x_pos_end;
        n_samples_seen += U16F16::from_num(1);
    }

    return;
}
