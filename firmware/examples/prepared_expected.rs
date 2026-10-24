//! Host oracle for the prepared-layer image: checksums/write counts per
//! case and mode from the same shared fixtures. Not target timing.
use overhead_s3_benchmark::{
    coast_measurement::{self, CASES, Screen},
    prepared_measurement::{self, MODES, Mode},
};
fn main() {
    println!("case,mode,checksum,writes,on_writes");
    for (name, lat, lon, fov) in CASES {
        let app = coast_measurement::app(name, lat, lon, fov);
        let mut layer = overhead_render::CoastLayer::new();
        for mode in MODES {
            let mut screen = Screen::new();
            prepared_measurement::run(&mut screen, &app, &mut layer, mode);
            if mode != Mode::Prepare {
                println!(
                    "{name},{mode:?},{},{},{}",
                    screen.checksum(),
                    screen.writes,
                    screen.on_writes
                );
            }
        }
    }
}
