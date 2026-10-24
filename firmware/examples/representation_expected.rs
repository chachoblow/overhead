//! Host oracle for the exact packed target / scene fixtures; not target timing.
use overhead_s3_benchmark::{
    coast_measurement::{self, CASES, Screen},
    representation_measurement,
};
fn main() {
    let directory = std::env::args()
        .nth(1)
        .expect("output directory (must not exist)");
    let directory = std::path::Path::new(&directory);
    std::fs::create_dir(directory).expect("fresh output directory");
    println!("case,mode,checksum,writes,on_writes");
    for (name, lat, lon, fov) in CASES {
        let app = coast_measurement::app(name, lat, lon, fov);
        for &mode in representation_measurement::modes(name, true) {
            let mut screen = Screen::new();
            assert!(representation_measurement::run(&mut screen, &app, mode));
            println!(
                "{name},{mode:?},{},{},{}",
                screen.checksum(),
                screen.writes,
                screen.on_writes
            );
            std::fs::write(
                directory.join(format!("{name}-{mode:?}.bin")),
                screen.pixels,
            )
            .unwrap();
        }
    }
}
