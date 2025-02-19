# Pico Environment Sensor

This firmware, written in Rust using embedded-hal ([rp235x-hal](https://crates.io/crates/rp235x-hal)), is designed for the [Raspberry Pi Pico 2 W](https://www.raspberrypi.com/documentation/microcontrollers/pico-series.html#pico2w-technical-specification) platform using the RP2350 microcontroller. Its application is an environment-sensing device which can measure CO2 concentration in the atmosphere, ambient temperature and humidity. 

## Building

### Prerequisites

- Rust compiler & cargo package manager: https://www.rust-lang.org/
- RPi Pico SDK: https://github.com/raspberrypi/pico-sdk
- RPi picotool: https://github.com/raspberrypi/picotool
- ARM bare metal compiler toolchain
    - `arm-none-eabi-gcc` (& `arm-none-eabi-newlib`) on Arch Linux, your system may have different package names

### Environment configuration

This build system assumes the environment variables: `PICO_BOARD`, `PICO_PLATFORM` & `PICO_SDK_PATH` are set. `picotool` should also be available in the `PATH`.

The [env-vars.sh](./scripts/env-vars.sh) script sets these values to `pico2_w`, `rp2350-arm-s` and my personal Pico SDK path, respectively.

### Compiling & running

- `cargo build` (or `cargo build --release`)

If you wish to run the binary on your Pico (connected in BOOTSEL mode):

- `cargo run` (or `cargo run --release`)
