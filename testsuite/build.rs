use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    // Put `memory.x` in the output directory and add it to the linker search path.
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());
    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(include_bytes!("memory.x"))
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");

    // `--nmagic` is required when memory section addresses are not aligned to 0x10000.
    println!("cargo:rustc-link-arg=--nmagic");

    // cortex-m-rt linker script.
    println!("cargo:rustc-link-arg=-Tlink.x");

    // embedded-test linker script + rust-analyzer "Run Test" button support.
    println!("cargo:rustc-link-arg=-Tembedded-test.x");
    println!("cargo:rustc-check-cfg=cfg(rust_analyzer)");

    // defmt linker script (defmt is always enabled in the testsuite).
    println!("cargo:rustc-link-arg=-Tdefmt.x");
}
