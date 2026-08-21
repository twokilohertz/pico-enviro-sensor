use core::fmt::Write;

use circular_buffer::FixedCircularBuffer;
use embassy_embedded_hal::shared_bus::blocking::spi::SpiDevice;
use embassy_rp::gpio::Output;
use embassy_sync::pubsub::WaitResult;
use embassy_time::{Duration, Timer};
use embedded_graphics::{
    Drawable,
    draw_target::DrawTarget,
    geometry::{Dimensions, Point, Size},
    mono_font::{MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::{Rgb565, RgbColor, WebColors},
    primitives::{Line, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, StyledDrawable},
    text::Text,
};
use embedded_graphics_framebuf::FrameBuf;
use fixed::{
    traits::Fixed,
    types::{I16F16, U16F16},
};
use heapless::String;
use mipidsi::{
    Builder,
    interface::SpiInterface,
    models::ST7789,
    options::{ColorInversion, Orientation, Rotation},
};

use embedded_graphics::mono_font::ascii::FONT_8X13 as MetricsFont;

use crate::{
    SpiBus0,
    messages::{Message, MessageBusSubscriber},
};

/// Display width (in pixels)
pub(crate) const PHYSICAL_DISPLAY_WIDTH: usize = 240;
/// Display height (in pixels)
pub(crate) const PHYSICAL_DISPLAY_HEIGHT: usize = 320;
/// Display's total number of pixels
pub(crate) const N_PIXELS: usize = PHYSICAL_DISPLAY_WIDTH * PHYSICAL_DISPLAY_HEIGHT;
/// Logical display width (in pixels), display is rotated from portrait to landscape
pub(crate) const LOGICAL_DISPLAY_WIDTH: usize = 320;
/// Logical display height (in pixels), display is rotated from portrait to landscape
pub(crate) const LOGICAL_DISPLAY_HEIGHT: usize = 240;

/// Determines the maximum refresh rate of the display, interval of 16.666 ms (60 fps)
const DISPLAY_MAX_UPDATE_INTERVAL: Duration = Duration::from_micros(16_667);

/// Size of the SPI display transfer buffer (in bytes)
const SPI_BUF_SIZE: usize = 512;

/// Maximum number of characters the formatted string buffers can hold
const MAX_STRING_BUFFER_SIZE: usize = 32;

/// Listens for sensor measurements on the [crate::messages::MessageBus] and outputs data to the display
#[embassy_executor::task]
pub async fn display_output(
    spi_bus: &'static SpiBus0,
    cs: Output<'static>,
    dc: Output<'static>,
    rst: Output<'static>,
    mut subscriber: MessageBusSubscriber<'static>,
) {
    // SPI bus/device initialisation
    let device = SpiDevice::new(spi_bus, cs);

    // Display initialisation
    let mut spi_buf: [u8; SPI_BUF_SIZE] = [0; SPI_BUF_SIZE];
    let interface = SpiInterface::new(device, dc, &mut spi_buf);

    let mut display = Builder::new(ST7789, interface)
        .reset_pin(rst)
        .display_size(
            PHYSICAL_DISPLAY_WIDTH as u16,
            PHYSICAL_DISPLAY_HEIGHT as u16,
        )
        // using the display in landscape mode (see LOGICAL_DISPLAY_xxx dimensions)
        .orientation(Orientation::new().rotate(Rotation::Deg270))
        // GMT020-02 IPS panel needs the colours inverted
        .invert_colors(ColorInversion::Inverted)
        .init(&mut embassy_time::Delay)
        .unwrap();

    // Framebuffer, prevents flickering on redraw (153.6 kB in Rgb565/u16 mode)
    let mut buf = [Rgb565::BLACK; N_PIXELS];
    let mut framebuf = FrameBuf::new(&mut buf, LOGICAL_DISPLAY_WIDTH, LOGICAL_DISPLAY_HEIGHT);
    // let framebuf_area = Rectangle::new(Point::zero(), framebuf.size());
    let framebuf_area = framebuf.bounding_box();

    // Initial clear to zero out the uninitialised display memory
    display
        .fill_contiguous(&framebuf_area, framebuf.data.iter().copied())
        .unwrap();

    // -- UI configuration & styles

    // Sizes & positions

    const UI_PADDING: Size = Size::new(2, 2);
    const PADDED_DISPLAY_WIDTH: u32 = LOGICAL_DISPLAY_WIDTH as u32 - UI_PADDING.width * 2;
    const GRAPH_BACKGROUND_SIZE: Size = Size::new(PADDED_DISPLAY_WIDTH, 74);
    const STATUS_BAR_SIZE: Size = Size::new(PADDED_DISPLAY_WIDTH, 10);
    const METRICS_TEXT_BASELINE: u32 = MetricsFont.baseline;

    // Graph backgrounds

    const TEMP_GRAPH_BACKGROUND_TOP_LEFT: Point = Point::new(
        0_i32.saturating_add_unsigned(UI_PADDING.width),
        0_i32.saturating_add_unsigned(UI_PADDING.height),
    );
    const HUMID_GRAPH_BACKGROUND_TOP_LEFT: Point = Point::new(
        TEMP_GRAPH_BACKGROUND_TOP_LEFT.x,
        TEMP_GRAPH_BACKGROUND_TOP_LEFT
            .y
            .saturating_add_unsigned(GRAPH_BACKGROUND_SIZE.height)
            .saturating_add_unsigned(UI_PADDING.height),
    );
    const CO2_GRAPH_BACKGROUND_TOP_LEFT: Point = Point::new(
        HUMID_GRAPH_BACKGROUND_TOP_LEFT.x,
        HUMID_GRAPH_BACKGROUND_TOP_LEFT
            .y
            .saturating_add_unsigned(GRAPH_BACKGROUND_SIZE.height)
            .saturating_add_unsigned(UI_PADDING.height),
    );
    const TEMP_GRAPH_BACKGROUND_RECT: Rectangle =
        Rectangle::new(TEMP_GRAPH_BACKGROUND_TOP_LEFT, GRAPH_BACKGROUND_SIZE);
    const HUMID_GRAPH_BACKGROUND_RECT: Rectangle =
        Rectangle::new(HUMID_GRAPH_BACKGROUND_TOP_LEFT, GRAPH_BACKGROUND_SIZE);
    const CO2_GRAPH_BACKGROUND_RECT: Rectangle =
        Rectangle::new(CO2_GRAPH_BACKGROUND_TOP_LEFT, GRAPH_BACKGROUND_SIZE);

    const STATUS_BAR_BACKGROUND_TOP_LEFT: Point = Point::new(
        0_i32.saturating_add_unsigned(UI_PADDING.width),
        ((LOGICAL_DISPLAY_HEIGHT - 1) as i32).saturating_sub_unsigned(STATUS_BAR_SIZE.height),
    );
    const STATUS_BAR_RECT: Rectangle =
        Rectangle::new(STATUS_BAR_BACKGROUND_TOP_LEFT, STATUS_BAR_SIZE);

    // Text

    const TEMP_TEXT_TOP_LEFT: Point = Point::new(
        TEMP_GRAPH_BACKGROUND_TOP_LEFT
            .x
            .saturating_add_unsigned(UI_PADDING.width),
        TEMP_GRAPH_BACKGROUND_TOP_LEFT
            .y
            .saturating_add_unsigned(METRICS_TEXT_BASELINE)
            .saturating_add_unsigned(UI_PADDING.height),
    );
    const TEMP_TEXT_DROP_SHADOW_TOP_LEFT: Point = Point::new(
        TEMP_TEXT_TOP_LEFT.x.saturating_add(1),
        TEMP_TEXT_TOP_LEFT.y.saturating_add(1),
    );

    const HUMID_TEXT_TOP_LEFT: Point = Point::new(
        HUMID_GRAPH_BACKGROUND_TOP_LEFT
            .x
            .saturating_add_unsigned(UI_PADDING.width),
        HUMID_GRAPH_BACKGROUND_TOP_LEFT
            .y
            .saturating_add_unsigned(METRICS_TEXT_BASELINE)
            .saturating_add_unsigned(UI_PADDING.height),
    );
    const HUMID_TEXT_DROP_SHADOW_TOP_LEFT: Point = Point::new(
        HUMID_TEXT_TOP_LEFT.x.saturating_add(1),
        HUMID_TEXT_TOP_LEFT.y.saturating_add(1),
    );

    const CO2_TEXT_TOP_LEFT: Point = Point::new(
        CO2_GRAPH_BACKGROUND_TOP_LEFT
            .x
            .saturating_add_unsigned(UI_PADDING.width),
        CO2_GRAPH_BACKGROUND_TOP_LEFT
            .y
            .saturating_add_unsigned(METRICS_TEXT_BASELINE)
            .saturating_add_unsigned(UI_PADDING.height),
    );
    const CO2_TEXT_DROP_SHADOW_TOP_LEFT: Point = Point::new(
        CO2_TEXT_TOP_LEFT.x.saturating_add(1),
        CO2_TEXT_TOP_LEFT.y.saturating_add(1),
    );

    // Colour schemes & styles
    // https://rgbcolorpicker.com/565 came in handy designing these colour schemes

    const GRAPH_BACKGROUND_COLOUR: Rgb565 = Rgb565::new(3, 7, 3);
    const GRAPH_BACKGROUND_STYLE: PrimitiveStyle<Rgb565> = PrimitiveStyleBuilder::new()
        .fill_color(GRAPH_BACKGROUND_COLOUR)
        .build();
    const STATUS_BAR_BACKGROUND_COLOUR: Rgb565 = Rgb565::new(3, 6, 3);
    const STATUS_BAR_BACKGROUND_STYLE: PrimitiveStyle<Rgb565> = PrimitiveStyleBuilder::new()
        .fill_color(STATUS_BAR_BACKGROUND_COLOUR)
        .build();

    const LINE_GRAPH_STYLE_BUILDER: PrimitiveStyleBuilder<Rgb565> =
        PrimitiveStyleBuilder::new().stroke_width(1);
    const TEMP_LINE_GRAPH_STYLE: PrimitiveStyle<Rgb565> = LINE_GRAPH_STYLE_BUILDER
        .stroke_color(Rgb565::CSS_GOLDENROD)
        .build();
    const HUMID_LINE_GRAPH_STYLE: PrimitiveStyle<Rgb565> = LINE_GRAPH_STYLE_BUILDER
        .stroke_color(Rgb565::CSS_SKY_BLUE)
        .build();
    const CO2_LINE_GRAPH_STYLE: PrimitiveStyle<Rgb565> = LINE_GRAPH_STYLE_BUILDER
        .stroke_color(Rgb565::CSS_SPRING_GREEN)
        .build();

    const TEXT_DROP_SHADOW_COLOUR: Rgb565 = Rgb565::new(2, 4, 2);
    const METRICS_TEXT_STYLE_BUILDER: MonoTextStyleBuilder<'_, Rgb565> =
        MonoTextStyleBuilder::<'_, Rgb565>::new().font(&MetricsFont);
    const TEMP_TEXT_STYLE: MonoTextStyle<'_, Rgb565> = METRICS_TEXT_STYLE_BUILDER
        .text_color(Rgb565::CSS_ORANGE)
        .build();
    const HUMID_TEXT_STYLE: MonoTextStyle<'_, Rgb565> = METRICS_TEXT_STYLE_BUILDER
        .text_color(Rgb565::CSS_AQUA)
        .build();
    const CO2_TEXT_STYLE: MonoTextStyle<'_, Rgb565> = METRICS_TEXT_STYLE_BUILDER
        .text_color(Rgb565::CSS_LAWN_GREEN)
        .build();
    const METRICS_TEXT_DROP_SHADOW_STYLE: MonoTextStyle<'_, Rgb565> = METRICS_TEXT_STYLE_BUILDER
        .text_color(TEXT_DROP_SHADOW_COLOUR)
        .build();

    /*
       Note about drawing positions:
       The embedded-graphics library follows the OpenGL convention of the
       top-left of the image being (0, 0) with X increasing to the right
       and Y increasing downwards.
    */

    // Sensor measurements

    /// Number of sensor measurement samples to store
    const MAX_N_SAMPLES: usize = 60;

    let mut temp_samples: FixedCircularBuffer<I16F16, MAX_N_SAMPLES> = Default::default();
    let mut humidity_samples: FixedCircularBuffer<U16F16, MAX_N_SAMPLES> = Default::default();
    let mut co2_samples: FixedCircularBuffer<U16F16, MAX_N_SAMPLES> = Default::default();

    // Text string buffers for formatted metrics

    let mut temp_text_buf = String::<MAX_STRING_BUFFER_SIZE>::new();
    let mut humidity_text_buf = String::<MAX_STRING_BUFFER_SIZE>::new();
    let mut co2_text_buf = String::<MAX_STRING_BUFFER_SIZE>::new();

    loop {
        Timer::after(DISPLAY_MAX_UPDATE_INTERVAL).await;

        // Wait for new sensor data

        match subscriber.next_message().await {
            WaitResult::Message(Message::SensorMeasurement(sensor_data)) => {
                temp_samples.push_back(sensor_data.temperature);
                humidity_samples.push_back(sensor_data.humidity);
                co2_samples.push_back(U16F16::saturating_from_num(sensor_data.co2)); // u16 -> U16F16

                // Graph drawing logic depends on the contiguous (i.e. single slice) invariant holding true
                temp_samples.make_contiguous();
                humidity_samples.make_contiguous();
                co2_samples.make_contiguous();

                // Format most recent measurement into the text buffers

                temp_text_buf.clear();
                humidity_text_buf.clear();
                co2_text_buf.clear();

                write!(
                    &mut temp_text_buf,
                    "Temperature: {:.1} C",
                    sensor_data.temperature
                )
                .unwrap();

                write!(
                    &mut humidity_text_buf,
                    "Humidity: {:.1} %",
                    sensor_data.humidity
                )
                .unwrap();

                write!(&mut co2_text_buf, "CO2: {} ppm", sensor_data.co2).unwrap();
            }
            WaitResult::Message(Message::Startup) => {
                core::hint::cold_path(); // will only happen once

                write!(&mut temp_text_buf, "Temperature: awaiting data ...",).unwrap();
                write!(&mut humidity_text_buf, "Humidity: awaiting data ...").unwrap();
                write!(&mut co2_text_buf, "CO2: awaiting data ...").unwrap();
            }
            _ => (),
        };

        // -- Drawing

        // Graph backgrounds

        TEMP_GRAPH_BACKGROUND_RECT
            .draw_styled(&GRAPH_BACKGROUND_STYLE, &mut framebuf)
            .unwrap();
        HUMID_GRAPH_BACKGROUND_RECT
            .draw_styled(&GRAPH_BACKGROUND_STYLE, &mut framebuf)
            .unwrap();
        CO2_GRAPH_BACKGROUND_RECT
            .draw_styled(&GRAPH_BACKGROUND_STYLE, &mut framebuf)
            .unwrap();

        // Status bar background

        STATUS_BAR_RECT
            .draw_styled(&STATUS_BAR_BACKGROUND_STYLE, &mut framebuf)
            .unwrap();

        // Line graphs

        line_graph(
            temp_samples.as_slices().0,
            &TEMP_GRAPH_BACKGROUND_RECT,
            &TEMP_LINE_GRAPH_STYLE,
            &mut framebuf,
        )
        .unwrap();

        line_graph(
            humidity_samples.as_slices().0,
            &HUMID_GRAPH_BACKGROUND_RECT,
            &HUMID_LINE_GRAPH_STYLE,
            &mut framebuf,
        )
        .unwrap();

        line_graph(
            co2_samples.as_slices().0,
            &CO2_GRAPH_BACKGROUND_RECT,
            &CO2_LINE_GRAPH_STYLE,
            &mut framebuf,
        )
        .unwrap();

        // Metrics summary text (& corresponding drop shadows)

        Text::new(
            &temp_text_buf,
            TEMP_TEXT_DROP_SHADOW_TOP_LEFT,
            METRICS_TEXT_DROP_SHADOW_STYLE,
        )
        .draw(&mut framebuf)
        .unwrap();
        Text::new(&temp_text_buf, TEMP_TEXT_TOP_LEFT, TEMP_TEXT_STYLE)
            .draw(&mut framebuf)
            .unwrap();

        Text::new(
            &humidity_text_buf,
            HUMID_TEXT_DROP_SHADOW_TOP_LEFT,
            METRICS_TEXT_DROP_SHADOW_STYLE,
        )
        .draw(&mut framebuf)
        .unwrap();
        Text::new(&humidity_text_buf, HUMID_TEXT_TOP_LEFT, HUMID_TEXT_STYLE)
            .draw(&mut framebuf)
            .unwrap();

        Text::new(
            &co2_text_buf,
            CO2_TEXT_DROP_SHADOW_TOP_LEFT,
            METRICS_TEXT_DROP_SHADOW_STYLE,
        )
        .draw(&mut framebuf)
        .unwrap();
        Text::new(&co2_text_buf, CO2_TEXT_TOP_LEFT, CO2_TEXT_STYLE)
            .draw(&mut framebuf)
            .unwrap();

        // Write the framebuffer's contents to the physical display & clear the framebuffer

        display
            .fill_contiguous(&framebuf_area, framebuf.data.iter().copied())
            .unwrap();
        framebuf.clear(Rgb565::BLACK).unwrap();
    }
}

/// Draw a line graph of a provided slice of measurement samples
///
/// **NOTE**: This function makes some assumptions that the length of the samples slice & bounding box dimensions will never exceed [u16::MAX]
fn line_graph<V, D>(
    samples: &[V],
    bounding_box: &Rectangle,
    line_style: &PrimitiveStyle<Rgb565>,
    target: &mut D,
) -> Result<(), D::Error>
where
    V: Fixed,
    D: DrawTarget<Color = Rgb565>,
{
    if samples.len() < 2 {
        core::hint::cold_path(); // Can't draw a line with less than two points, only happens before there's 2 measurements made
        return Ok(());
    }

    let samples_min = *samples.iter().min().unwrap();
    let samples_max = *samples.iter().max().unwrap();

    let n_pairs = samples.array_windows::<2>().count();

    let mut prev_line_end: Option<Point> = None;

    for (i, [a, b]) in samples.array_windows::<2>().copied().enumerate() {
        let start_y = remap_fixed(
            a,
            samples_min,
            samples_max,
            V::saturating_from_num(
                bounding_box
                    .top_left
                    .y
                    .saturating_add_unsigned(bounding_box.size.height - 1),
            ),
            V::saturating_from_num(bounding_box.top_left.y),
        );

        let end_y = remap_fixed(
            b,
            samples_min,
            samples_max,
            V::saturating_from_num(
                bounding_box
                    .top_left
                    .y
                    .saturating_add_unsigned(bounding_box.size.height - 1),
            ),
            V::saturating_from_num(bounding_box.top_left.y),
        );

        let line_start = if let Some(prev) = prev_line_end {
            // Don't recalculate the start point as we've already calculated it in the previous iteration
            prev
        } else {
            core::hint::cold_path(); // Only happens on the first iteration

            let start_x = remap_fixed(
                U16F16::saturating_from_num(i),
                U16F16::saturating_from_num(0),
                U16F16::saturating_from_num(n_pairs),
                U16F16::saturating_from_num(bounding_box.top_left.x),
                U16F16::saturating_from_num(
                    bounding_box
                        .top_left
                        .x
                        .saturating_add_unsigned(bounding_box.size.width - 1),
                ),
            );

            Point::new(start_x.saturating_to_num(), start_y.saturating_to_num())
        };

        let line_end = {
            let end_x = remap_fixed(
                U16F16::saturating_from_num(i + 1),
                U16F16::saturating_from_num(0),
                U16F16::saturating_from_num(n_pairs),
                U16F16::saturating_from_num(bounding_box.top_left.x),
                U16F16::saturating_from_num(
                    bounding_box
                        .top_left
                        .x
                        .saturating_add_unsigned(bounding_box.size.width - 1),
                ),
            );

            Point::new(end_x.saturating_to_num(), end_y.saturating_to_num())
        };

        Line::new(line_start, line_end).draw_styled(line_style, target)?;

        prev_line_end = Some(line_end);
    }

    Ok(())
}

fn remap_fixed<T>(val: T, in_start: T, in_end: T, out_start: T, out_end: T) -> T
where
    T: Fixed,
{
    if in_start == in_end {
        // Early return to prevent a divide-by-zero
        core::hint::cold_path();
        return out_start;
    }

    let in_diff = T::Signed::saturating_from_num(in_end) - T::Signed::saturating_from_num(in_start);
    let out_diff =
        T::Signed::saturating_from_num(out_end) - T::Signed::saturating_from_num(out_start);
    let val_diff = T::Signed::saturating_from_num(val) - T::Signed::saturating_from_num(in_start);
    let ratio = val_diff / in_diff;

    T::saturating_from_num(T::Signed::saturating_from_num(out_start) + (ratio * out_diff))
}
