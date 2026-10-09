use std::{env, fmt::Write, fs, path::PathBuf, process::Command};

use sgp4::{
    Elements,
    chrono::{Datelike, NaiveDateTime, TimeDelta, Timelike},
};

#[path = "src/selection.rs"]
mod selection;

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
    println!("cargo:rerun-if-changed=src/selection.rs");
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
    // Prepare age-shifted grids on the host, never inside target timing or on
    // its stack. Zero age preserves the original host/target workload.
    for (index, days) in selection::AGE_DAYS.into_iter().enumerate() {
        let name = if days == 0 {
            "TIMES".into()
        } else {
            format!("TIMES_{index}")
        };
        writeln!(
            source,
            "pub static {name}: [[NaiveDateTime; TRACK_STEPS]; 4] = ["
        )
        .unwrap();
        for e in &elements {
            source.push_str("[\n");
            for i in 0..1441 {
                writeln!(
                    source,
                    "{},",
                    datetime(
                        e.datetime + TimeDelta::days(days) - TimeDelta::hours(12)
                            + TimeDelta::seconds(i * 60)
                    )
                )
                .unwrap();
            }
            source.push_str("],\n");
        }
        source.push_str("];\n");
    }
    // Each grid is a separate static so unused ages can be linker-collected.
    source.push_str(
        "fn age_times(days: i64) -> &'static [[NaiveDateTime; TRACK_STEPS]; 4] { match days {\n",
    );
    for (index, days) in selection::AGE_DAYS.into_iter().enumerate() {
        let name = if days == 0 {
            "TIMES".into()
        } else {
            format!("TIMES_{index}")
        };
        writeln!(source, "{days} => &{name},").unwrap();
    }
    source.push_str("_ => panic!(\"unsupported experimental age\"), } }\n");
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("fixtures.rs"),
        source,
    )
    .unwrap();
    for (name, default) in [("BENCH_SUITE", "baseline"), ("BENCH_SAMPLES", "5")] {
        println!("cargo:rerun-if-env-changed={name}");
        let value = env::var(name).unwrap_or_else(|_| default.into());
        if name == "BENCH_SAMPLES" {
            assert!(
                selection::sample_count(&value).is_some(),
                "BENCH_SAMPLES must be 1..5"
            );
        } else {
            assert!(
                selection::SUITE_NAMES.contains(&value.as_str()),
                "unknown BENCH_SUITE: {value}"
            );
        }
        println!("cargo:rustc-env={name}={value}");
    }
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
