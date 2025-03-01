#![no_std]
#![no_main]

mod sensor_task;

use core::cell::RefCell;

use rtt_target::{rprintln, rtt_init_print};

use embassy_executor::Spawner;
use embassy_rp::{block::ImageDef, i2c::Blocking, i2c::I2c, peripherals::I2C0};
use embassy_sync::blocking_mutex::{raw::NoopRawMutex, Mutex};
use embassy_time::Timer;
use static_cell::StaticCell;

use sensor_task::sensor_read_task;

embassy_rp::bind_interrupts!(struct Irqs {
    I2C0_IRQ => embassy_rp::i2c::InterruptHandler<embassy_rp::peripherals::I2C0>;
});

type I2c0BusType = Mutex<NoopRawMutex, RefCell<I2c<'static, I2C0, Blocking>>>;

/// Entrypoint
#[embassy_executor::main]
async fn main(spawner: Spawner) {
    // Initialise RTT logging

    rtt_init_print!();
    rprintln!("RTT logging initialised");

    let peripherals = embassy_rp::init(Default::default());

    // Initialise I2C0 on pins 4 & 5 for the SCD41 sensor
    let sda = peripherals.PIN_4;
    let scl = peripherals.PIN_5;
    let mut i2c_config = embassy_rp::i2c::Config::default();
    i2c_config.frequency = 400_000u32; // 400 kHz
    let i2c_bus = embassy_rp::i2c::I2c::new_blocking(peripherals.I2C0, scl, sda, i2c_config);
    static I2C0_BUS: StaticCell<I2c0BusType> = StaticCell::new();
    let shared_i2c0_bus = I2C0_BUS.init(Mutex::new(RefCell::new(i2c_bus)));

    spawner.must_spawn(sensor_read_task(shared_i2c0_bus));

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
