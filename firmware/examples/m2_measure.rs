#![cfg_attr(target_arch = "xtensa", no_std)]
#![cfg_attr(target_arch = "xtensa", no_main)]

#[cfg(all(target_arch = "xtensa", feature = "catalogue-memory"))]
#[path = "m2_support/target.rs"]
mod target;

#[cfg(all(target_arch = "xtensa", feature = "catalogue-memory"))]
#[esp_hal::main]
fn main() -> ! {
    target::run(false)
}

#[cfg(all(target_arch = "xtensa", not(feature = "catalogue-memory")))]
compile_error!("m2_measure requires --features catalogue-memory");

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("Target-only measurement example; run library tests for host pipeline validation.");
}
