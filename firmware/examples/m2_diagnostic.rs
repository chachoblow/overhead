#![cfg_attr(target_arch = "xtensa", no_std)]
#![cfg_attr(target_arch = "xtensa", no_main)]

#[cfg(all(
    target_arch = "xtensa",
    feature = "catalogue-memory",
    feature = "numerical-reference"
))]
#[path = "m2_support/target.rs"]
mod target;

#[cfg(all(
    target_arch = "xtensa",
    feature = "catalogue-memory",
    feature = "numerical-reference"
))]
#[esp_hal::main]
fn main() -> ! {
    target::run(true)
}

#[cfg(all(
    target_arch = "xtensa",
    not(all(feature = "catalogue-memory", feature = "numerical-reference"))
))]
compile_error!("m2_diagnostic requires --features catalogue-memory,numerical-reference");

#[cfg(not(target_arch = "xtensa"))]
fn main() {
    eprintln!("Target-only diagnostic example; run numerical_expected for host numerical checks.");
}
