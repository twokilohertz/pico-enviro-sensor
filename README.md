# Pico Environment Sensor

This firmware, written in Rust using [Embassy](https://github.com/embassy-rs/embassy), is designed for the [Raspberry Pi Pico 2 W](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html#wireless_pico2) platform using the RP2350 microcontroller. Its application is an environment-sensing device which can measure CO₂ concentration in the atmosphere, ambient temperature and humidity. 

The effects of increased CO₂ concentration on cognitive function becomes increasingly pronounced at CO₂ concentrations exceeding 1000 ppm. There's a fantastic [YouTube video](https://www.youtube.com/watch?v=1Nh_vxpycEA) by Kurtis Baute and Tom Scott which you can watch for a short summary. Open your window right now!

## Hardware

- Microcontroller: [Raspberry Pi Pico 2 W (RP2350)](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html#wireless_pico2)
  - Datasheet (board): https://pip.raspberrypi.com/documents/RP-008304-DS
  - Datasheet (chip): https://pip.raspberrypi.com/documents/RP-008373-DS
- Environment sensor: [Sensirion SCD41](https://sensirion.com/products/catalog/SCD41)
  - Datasheet: https://sensirion.com/resource/datasheet/scd4x
- Display: [GoldenMorning GMT020-02 ST7789V](https://goldenmorninglcd.com/tft-display-module/2-inch-240x320-st7789v-gmt020-02/)
  - Datasheet (display): https://goldenmorninglcd.com/wp-content/uploads/2023/08/GMT020-02.pdf
  - Datasheet (driver): https://wiki.pine64.org/images/5/54/ST7789V_v1.6.pdf

## Building

### Prerequisites

- Rust compiler & cargo package manager: https://www.rust-lang.org/
- [probe-rs](https://probe.rs/) for programming the device, receiving RTT logging output & debugging
- GCC ARM bare metal compiler toolchain
    - `arm-none-eabi-gcc` (& `arm-none-eabi-newlib`) on Arch Linux, your system may have different package names
- The ARMv8 Rust toolchain: `rustup target add thumbv8m.main-none-eabihf`

### Notes

This project's [`Cargo.toml`](./Cargo.toml) includes a custom build profile ("`dist`", and a corresponding "`dist-dev`" profile retaining symbols) which optimises the executable for maximum performance at the cost of slower build times. This profile is designed for flashing a final build onto an end user's device.

**The project's build profile also overrides the default Rust "`dev`" profile with `opt-level = "s"`.** If this is undesirable for debugging purposes - as the compiler can optimise out quite a lot of useful information in this mode - it is recommended to remove this optimisation directive.

By install probe-rs as mentioned in the prerequisities you will have access to `cargo flash` and `cargo embed` for flashing the executable onto the device.

### Compiling & running

- `cargo build` (or `cargo build --release` / `cargo build --profile dist`)

If you wish to run the binary on your Pico (connected in BOOTSEL mode):

- `cargo run` (or `cargo run --release` / `cargo run --profile dist`)

## Hacking & debugging

This project uses probe-rs for flashing & debugging, though one may use `picotool` and `OpenOCD` instead if you so wish. Ensure that you have a debug probe which supports to RP235x series of chips and supports the SWD protocol. I use the [Raspberry Pi debug probe](https://www.raspberrypi.com/documentation/microcontrollers/debug-probe.html) (USB VID:PID `2e8a:000c`) for debugging the device using the on-board debugging pins. 
