#![no_std]
#![no_main]

use core::panic::PanicInfo;

use rtt_target::{rprintln, rtt_init_print};

use embassy_executor::Spawner;
use embassy_rp::block::ImageDef;

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    rtt_init_print!();
    rprintln!("RTT logging initialised");

    let _peripherals = embassy_rp::init(Default::default());

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    rprintln!("Panicked! {}", info);

    loop {
        unsafe {
            core::arch::asm!("wfi");
        }
    }
}

#[link_section = ".start_block"]
#[used]
static IMAGE_DEF: ImageDef = ImageDef::secure_exe();

// Program metadata for picotool
#[link_section = ".bi_entries"]
#[used]
pub static PICOTOOL_ENTRIES: [embassy_rp::binary_info::EntryAddr; 4] = [
    embassy_rp::binary_info::rp_program_name!(c"Pico Environment Sensor"),
    embassy_rp::binary_info::rp_program_description!(
        c"A CO2, temperature & humidity sensing application for the RPi Pico 2 W"
    ),
    embassy_rp::binary_info::rp_cargo_version!(),
    embassy_rp::binary_info::rp_program_build_attribute!(),
];
