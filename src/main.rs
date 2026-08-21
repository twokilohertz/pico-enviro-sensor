#![no_std]
#![no_main]

use core::{cell::RefCell, panic};

use embassy_executor::Spawner;
use embassy_rp::{
    block::ImageDef,
    gpio::{Level, Output},
    i2c,
    peripherals::{I2C0, SPI0},
    spi,
};
use embassy_sync::blocking_mutex::{Mutex, raw::NoopRawMutex};
use rtt_target::debug_rprintln;
use static_cell::StaticCell;

use crate::messages::{Message, MessageBus};

mod messages;
mod tasks;

// Not currently using multiple executors & only one device is on the SPI/I2C bus
type SpiBus0 = Mutex<NoopRawMutex, RefCell<spi::Spi<'static, SPI0, spi::Blocking>>>;
type I2cBus0 = Mutex<NoopRawMutex, RefCell<i2c::I2c<'static, I2C0, i2c::Blocking>>>;

/// Entrypoint
#[embassy_executor::main()]
async fn main(spawner: Spawner) {
    // Initialise logging (debug mode)

    rtt_target::debug_rtt_init_print!();
    debug_rprintln!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));

    let p = embassy_rp::init(Default::default());

    // -- Configure peripherals

    // I2C sensor configuration

    let i2c_config = {
        let mut cfg = i2c::Config::default();
        cfg.frequency = 400_000; // 400 KHz
        cfg
    };

    let sda = p.PIN_12;
    let scl = p.PIN_13;
    let i2c = embassy_rp::i2c::I2c::new_blocking(p.I2C0, scl, sda, i2c_config);
    static I2C0_BUS: StaticCell<I2cBus0> = StaticCell::new();
    let i2c_bus = I2C0_BUS.init(Mutex::new(RefCell::new(i2c)));

    // SPI display configuration

    let spi_peripheral = p.SPI0;
    let spi_clk = p.PIN_18;
    let spi_mosi = p.PIN_19;
    let spi_cs = Output::new(p.PIN_21, Level::High);

    let dc = Output::new(p.PIN_16, Level::Low);
    let rst = Output::new(p.PIN_17, Level::High);

    let spi_config = {
        let mut cfg = spi::Config::default();
        cfg.frequency = 60_000_000; // theorhetical ~62.5 MHz maximum (60 MHz for safety)
        cfg
    };

    let spi = spi::Spi::new_blocking_txonly(spi_peripheral, spi_clk, spi_mosi, spi_config);

    static SPI_BUS: StaticCell<SpiBus0> = StaticCell::new();
    let spi_bus = SPI_BUS.init(Mutex::new(spi.into()));

    // -- Invoke tasks

    // System-wide message bus
    static MESSAGE_BUS: StaticCell<MessageBus> = StaticCell::new();
    let message_bus = MESSAGE_BUS.init(MessageBus::new());

    // Sensor read
    spawner.spawn(tasks::sensor::sensor_read(i2c_bus, message_bus.publisher().unwrap()).unwrap());

    // Display output
    spawner.spawn(
        tasks::display::display_output(spi_bus, spi_cs, dc, rst, message_bus.subscriber().unwrap())
            .unwrap(),
    );

    // Alert all tasks listening on the message bus that the system has initialised & started

    message_bus
        .publisher()
        .unwrap()
        .publish(Message::Startup)
        .await;
}

#[panic_handler]
fn panic(panic: &panic::PanicInfo) -> ! {
    debug_rprintln!("{}", panic);
    cortex_m::asm::udf();
}

/// Executable type header for the RP2350 bootloader
#[unsafe(link_section = ".start_block")]
#[used]
pub static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

/// Program metadata for picotool
#[unsafe(link_section = ".bi_entries")]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 6] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico Enviro Sensor"),
    embassy_rp::binary_info::rp_program_description!(unsafe {
        core::ffi::CStr::from_bytes_with_nul_unchecked(
            concat!(env!("CARGO_PKG_DESCRIPTION"), "\0").as_bytes(), // See: https://github.com/rp-rs/rp-hal/pull/1003
        )
    }),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
    embassy_rp::binary_info::rp_cargo_homepage_url!(),
    embassy_rp::binary_info::rp_pico_board!(c"Raspberry Pi Pico 2 W"),
];
