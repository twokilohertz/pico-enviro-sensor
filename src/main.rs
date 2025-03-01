#![no_std]
#![no_main]

mod display_task;
mod sensor_task;

use core::cell::RefCell;

use rtt_target::{rprintln, rtt_init_print};

use embassy_executor::Spawner;
use embassy_rp::{
    block::ImageDef,
    gpio::{Level, Output},
    i2c,
    peripherals::{I2C0, SPI0},
    spi,
};
use embassy_sync::{
    blocking_mutex::{
        raw::{CriticalSectionRawMutex, NoopRawMutex},
        Mutex,
    },
    signal::Signal,
};
use embassy_time::Timer;
use static_cell::StaticCell;

use display_task::display_output_task;
use sensor_task::sensor_read_task;

type I2c0BusMutex = Mutex<NoopRawMutex, RefCell<i2c::I2c<'static, I2C0, i2c::Blocking>>>;
type Spi0BusMutex = Mutex<NoopRawMutex, RefCell<spi::Spi<'static, SPI0, spi::Blocking>>>;

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

static SENSOR_DATA_SIGNAL: Signal<CriticalSectionRawMutex, scd4x::types::SensorData> =
    Signal::new();

/// Entrypoint
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialise RTT logging

    rtt_init_print!();
    rprintln!("RTT logging initialised");

    let peripherals = embassy_rp::init(Default::default());

    // Initialise I2C0 (SDA: pin 4, SCL: pin 5)
    let sda = peripherals.PIN_4;
    let scl = peripherals.PIN_5;
    let mut i2c_config = embassy_rp::i2c::Config::default();
    i2c_config.frequency = 400_000u32; // 400 kHz
    let i2c_bus = embassy_rp::i2c::I2c::new_blocking(peripherals.I2C0, scl, sda, i2c_config);
    static I2C0_BUS: StaticCell<I2c0BusMutex> = StaticCell::new();
    let shared_i2c0_bus = I2C0_BUS.init(Mutex::new(RefCell::new(i2c_bus)));

    // Start new task for reading data from the sensor
    spawner.must_spawn(sensor_read_task(shared_i2c0_bus));

    // Initialise SPI0 (MOSI: pin 19, SCLK: pin 18, CS: pin 17, DC: pin 14, RST: pin 15)
    let mosi = peripherals.PIN_19;
    let sclk = peripherals.PIN_18;
    let cs = Output::new(peripherals.PIN_17, Level::Low);
    let dc = Output::new(peripherals.PIN_14, Level::Low);
    static SPI0_RST_PIN: StaticCell<Output<'_>> = StaticCell::new(); // Initialised before launching task
    let rst = SPI0_RST_PIN.init(Output::new(peripherals.PIN_15, Level::Low));

    let mut spi_config = spi::Config::default();
    spi_config.frequency = 4_000_000u32; // 4 MHz
    let spi_bus = spi::Spi::new_blocking_txonly(peripherals.SPI0, sclk, mosi, spi_config.clone());
    static SPI0_BUS: StaticCell<Spi0BusMutex> = StaticCell::new();
    let shared_spi0_bus = SPI0_BUS.init(Mutex::new(RefCell::new(spi_bus)));

    // Start new task for outputting to the display
    spawner.must_spawn(display_output_task(
        shared_spi0_bus,
        cs,
        dc,
        rst,
        spi_config.clone(),
    ));

    loop {
        Timer::after_secs(1).await;
    }
}

/// Panic handler
#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    rprintln!("Panicked! {}", info);

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

/// Executable type header for the RP2350 bootloader
#[link_section = ".start_block"]
#[used]
static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

/// Program metadata for picotool
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico Environment Sensor"),
    embassy_rp::binary_info::rp_program_description!(
        c"CO2, temperature & humidity sensing application for the RPi Pico 2 W"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];
