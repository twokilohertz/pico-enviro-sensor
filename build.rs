use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    println!("cargo:rustc-link-search={}", out.display());

    #[cfg(not(feature = "skip-cyw43-firmware"))]
    download_cyw43_firmware();

    // ARM build

    let memory_x = include_bytes!("memory.x");
    let mut f = File::create(out.join("memory.x")).unwrap();
    f.write_all(memory_x).unwrap();
    println!("cargo:rerun-if-changed=memory.x");

    // RISC-V build

    let rp235x_riscv_x = include_bytes!("rp235x_riscv.x");
    let mut f = File::create(out.join("rp235x_riscv.x")).unwrap();
    f.write_all(rp235x_riscv_x).unwrap();
    println!("cargo:rerun-if-changed=rp235x_riscv.x");

    println!("cargo:rerun-if-changed=build.rs");
}

/// Downloads to firmware for the Wi-Fi / Bluetooth chipset on the RPi Pico 2 W
#[cfg(not(feature = "skip-cyw43-firmware"))]
fn download_cyw43_firmware() {
    let download_folder = "cyw43-firmware";
    let url_base = "https://raw.githubusercontent.com/embassy-rs/embassy/refs/tags/cyw43-v0.3.0/cyw43-firmware";
    let file_names = [
        "43439A0.bin",
        "43439A0_btfw.bin",
        "43439A0_clm.bin",
        "LICENSE-permissive-binary-license-1.0.txt",
        "README.md",
    ];

    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed={}", download_folder);
    std::fs::create_dir_all(download_folder).expect("failed to create firmware directory");

    // download each file into the directory "cyw43-firmware"
    for file in file_names {
        let url = format!("{}/{}", url_base, file);
        // only fetch if it doesn't exist
        if std::path::Path::new(download_folder).join(file).exists() {
            continue;
        }
        match reqwest::blocking::get(&url) {
            Ok(response) => {
                let content = response.bytes().expect("failed to read file content");
                let file_path = PathBuf::from(download_folder).join(file);
                std::fs::write(file_path, &content).expect("failed to write file");
            }
            Err(err) => panic!(
                "failed to download the cyw43 firmware from {}: {}, required for BLE support",
                url, err
            ),
        }
    }
}
