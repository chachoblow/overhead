use std::{env, fmt::Write, fs, path::PathBuf, process::Command};

use sgp4::{
    Elements,
    chrono::{Datelike, NaiveDateTime, TimeDelta, Timelike},
};

fn datetime(time: NaiveDateTime) -> String {
    format!(
        "NaiveDate::from_ymd_opt({}, {}, {}).unwrap().and_hms_nano_opt({}, {}, {}, {}).unwrap()",
        time.year(),
        time.month(),
        time.day(),
        time.hour(),
        time.minute(),
        time.second(),
        time.nanosecond()
    )
}

fn main() {
    let iss_path = "../core/tests/fixtures/iss-25544.json";
    let tle_path = "../tools/fixtures/pass-intervals.tle";
    println!("cargo:rerun-if-changed={iss_path}");
    println!("cargo:rerun-if-changed={tle_path}");
    println!("cargo:rerun-if-changed=build.rs");
    let mut elements: Vec<Elements> =
        serde_json::from_str(&fs::read_to_string(iss_path).unwrap()).unwrap();
    assert_eq!(elements.len(), 1);
    let tle = fs::read_to_string(tle_path).unwrap();
    let lines: Vec<_> = tle.lines().collect();
    assert_eq!(lines.len(), 9);
    for record in lines.as_chunks::<3>().0 {
        elements
            .push(Elements::from_tle(None, record[1].as_bytes(), record[2].as_bytes()).unwrap());
    }
    // Numeric fields are emitted losslessly. Allocation-only metadata is not
    // needed for propagation. Keep reading the original fixtures, never a copy.
    let mut source = String::from(
        "use overhead_core::sgp4::{Elements, Classification, chrono::NaiveDate};\nfn elements() -> [Elements; 4] { [\n",
    );
    for e in &elements {
        assert_eq!(e.classification, sgp4::Classification::Unclassified);
        writeln!(source, "Elements {{ norad_id: {}, classification: Classification::Unclassified, datetime: {}, element_set_number: {}, revolution_number: {}, ephemeris_type: {},",
            e.norad_id, datetime(e.datetime), e.element_set_number, e.revolution_number, e.ephemeris_type).unwrap();
        macro_rules! float_fields {
            ($($field:ident),* $(,)?) => { $(
                writeln!(source, "{}: f64::from_bits({}),", stringify!($field), e.$field.to_bits()).unwrap();
            )* };
        }
        float_fields!(
            mean_motion_dot,
            mean_motion_ddot,
            drag_term,
            inclination,
            right_ascension,
            eccentricity,
            argument_of_perigee,
            mean_anomaly,
            mean_motion
        );
        source.push_str("},\n");
    }
    source.push_str("] }\n");
    // Match the host benchmark's precomputed timestamp grid, but store it in
    // flash rodata rather than allocating 69 KiB of target heap/stack.
    source.push_str("pub static TIMES: [[NaiveDateTime; TRACK_STEPS]; 4] = [\n");
    for e in &elements {
        source.push_str("[\n");
        for i in 0..1441 {
            writeln!(
                source,
                "{},",
                datetime(e.datetime - TimeDelta::hours(12) + TimeDelta::seconds(i * 60))
            )
            .unwrap();
        }
        source.push_str("],\n");
    }
    source.push_str("];\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("fixtures.rs"),
        source,
    )
    .unwrap();
    let compiler = Command::new(env::var_os("RUSTC").unwrap())
        .arg("--version")
        .output()
        .unwrap();
    assert!(compiler.status.success());
    println!(
        "cargo:rustc-env=BENCH_RUSTC={}",
        String::from_utf8(compiler.stdout).unwrap().trim()
    );
}
