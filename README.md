# Pico Environment Sensor

This firmware, written in Rust using [embassy-rs](https://github.com/embassy-rs/embassy), is designed for the [Raspberry Pi Pico 2 W](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html#pico2w-technical-specification) platform using the RP2350 microcontroller. Its application is an environment-sensing device which can measure CO₂ concentration in the atmosphere, ambient temperature and humidity. 

The effects of increased CO₂ concentration on cognitive function become increasingly pronounced at CO₂ concentrations exceeding 1000 ppm. There's a fantastic [YouTube video](https://www.youtube.com/watch?v=1Nh_vxpycEA) by Kurtis Baute and Tom Scott which you can watch for a short summary. Open your window right now!

## Building

### Prerequisites

- Rust compiler & cargo package manager: https://www.rust-lang.org/
- [probe-rs](https://probe.rs/) for programming the device, receiving RTT logging output & debugging
- ARM bare metal compiler toolchain
    - `arm-none-eabi-gcc` (& `arm-none-eabi-newlib`) on Arch Linux, your system may have different package names
- The ARMv8 or RISC-V Rust toolchain:
  - ARMv8: `rustup target add thumbv8m.main-none-eabihf`
  - RISC-V: `rustup target add riscv32imac-unknown-none-elf`

### Notes

This project's [`Cargo.toml`](./Cargo.toml) includes a custom build profile ("`dist`") which optimises the executable for maximum performance at the cost of slower build times. This profile is designed for flashing a final build onto an end user's device.

By install probe-rs as mentioned in the prerequisities you will have access to `cargo flash` and `cargo embed` for flashing the executable onto the device.

### Compiling & running

- `cargo build` (or `cargo build --release` / `cargo build --profile dist`)

If you wish to run the binary on your Pico (connected in BOOTSEL mode):

- `cargo run` (or `cargo run --release` / `cargo run --profile dist`)

## Hacking & debugging

This project uses probe-rs for flashing & debugging, though one may use `picotool` and `OpenOCD` instead if you so wish. Ensure that you have a debug probe which supports to RP235x series of chips and supports the SWD protocol. I use the Raspberry Pi debug probe (USB VID:PID `2e8a:000c`) for debugging the device using the on-board debugging pins. 

## TODO / Future improvements

- Companion smartphone app using BLE to receive measurement data from the device for notifications & further analysis/tracking
- Low-power state, reducing screen brightness, sensor sampling rate and putting periphals to sleep
- 3D printed case housing the MCU, sensor, display and rechargable lithium ion battery
