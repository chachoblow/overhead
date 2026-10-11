//! Host-only conversion of pinned independent data; no propagation/geometry here.
use serde_json::Value;
use sgp4::{Elements, chrono::NaiveDateTime};
use std::{env, fmt::Write, fs, path::PathBuf};

fn number(v: &Value) -> String {
    let n = v.as_f64().expect("numeric fixture value");
    assert!(n.is_finite());
    format!("f64::from_bits({})", n.to_bits())
}
fn vector(v: &Value) -> String {
    assert_eq!(v.as_array().unwrap().len(), 3);
    format!("[{}, {}, {}]", number(&v[0]), number(&v[1]), number(&v[2]))
}
fn site(v: &Value) -> String {
    format!(
        "[{}, {}, {}]",
        number(&v["latitude_deg"]),
        number(&v["longitude_deg"]),
        number(&v["altitude_km"])
    )
}
fn time(v: &Value) -> String {
    let t = NaiveDateTime::parse_from_str(v.as_str().unwrap(), "%Y-%m-%dT%H:%M:%S%.f").unwrap();
    super::datetime(t)
}
fn elements(source: &mut String, e: &Elements) {
    assert_eq!(e.classification, sgp4::Classification::Unclassified);
    writeln!(source, "Elements {{ norad_id: {}, classification: Classification::Unclassified, datetime: {}, element_set_number: {}, revolution_number: {}, ephemeris_type: {},",
        e.norad_id, super::datetime(e.datetime), e.element_set_number, e.revolution_number, e.ephemeris_type).unwrap();
    if env::var_os("CARGO_FEATURE_CATALOGUE_MEMORY").is_some() {
        source.push_str("object_name: None, international_designator: None,\n");
    }
    macro_rules! floats {
        ($($field:ident),*) => { $(writeln!(source, "{}: f64::from_bits({}),", stringify!($field), e.$field.to_bits()).unwrap();)* };
    }
    floats!(
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
    source.push_str("}\n");
}

pub fn generate() {
    let path = "../tools/fixtures/m2-numerical/references.json";
    let iss = "../core/tests/fixtures/iss-25544.json";
    println!("cargo:rerun-if-changed={path}");
    println!("cargo:rerun-if-changed={iss}");
    let data: Value = serde_json::from_str(&fs::read_to_string(path).unwrap()).unwrap();
    let mut source = String::from(
        "// Generated from pinned references; do not edit.\nuse overhead_core::sgp4::{Elements, Classification, chrono::NaiveDate};\nfn reference_elements(index: usize) -> Elements { match index {\n",
    );
    let teme = data["teme"].as_array().unwrap();
    assert_eq!(teme.len(), 12);
    for (i, id) in [5, 8195, 24208, 28129].into_iter().enumerate() {
        let row = &teme[i * 3];
        let e = Elements::from_tle(
            None,
            row["tle"][0].as_str().unwrap().as_bytes(),
            row["tle"][1].as_str().unwrap().as_bytes(),
        )
        .unwrap();
        assert_eq!(e.norad_id, id);
        for (offset, minutes) in [0, 360, 1440].into_iter().enumerate() {
            let r = &teme[i * 3 + offset];
            assert_eq!(r["norad_id"], id);
            assert_eq!(r["minutes"], minutes);
            assert_eq!(r["tle"], row["tle"]);
        }
        writeln!(source, "{i} => ").unwrap();
        elements(&mut source, &e);
        source.push_str(",\n");
    }
    let iss: Vec<Elements> = serde_json::from_str(&fs::read_to_string(iss).unwrap()).unwrap();
    assert_eq!(iss.len(), 1);
    assert_eq!(iss[0].norad_id, 25544);
    source.push_str("4 => ");
    elements(&mut source, &iss[0]);
    source.push_str(", _ => panic!(\"reference index\"), } }\n");
    source.push_str("static TEME: [StateReference; 12] = [\n");
    for r in teme {
        writeln!(
            source,
            "StateReference {{ minutes: {}, position: {}, velocity: {} }},",
            r["minutes"],
            vector(&r["position_km"]),
            vector(&r["velocity_km_s"])
        )
        .unwrap();
    }
    source.push_str("];\n");
    // Date construction is deferred, not an allocated/parsing runtime path.
    for (key, count, ty, name) in [
        ("rotations", 9, "RotationReference", "rotation"),
        ("geodetic", 13, "GeodeticReference", "geodetic"),
        ("geometry", 27, "ObserverReference", "geometry"),
        ("iss_pipeline", 12, "ObserverReference", "pipeline"),
    ] {
        let rows = data[key].as_array().unwrap();
        assert_eq!(rows.len(), count);
        writeln!(
            source,
            "fn {name}_reference(index: usize) -> {ty} {{ match index {{"
        )
        .unwrap();
        for (i, r) in rows.iter().enumerate() {
            let fields = match key {
                "rotations" => format!(
                    "time: {}, teme: {}, ecef: {}",
                    time(&r["utc"]),
                    vector(&r["teme_km"]),
                    vector(&r["ecef_km"])
                ),
                "geodetic" => format!("site: {}, ecef: {}", site(r), vector(&r["ecef_km"])),
                _ => format!(
                    "site: {}, target: {}, time: {}, range: {}, azimuth_deg: {}, elevation_deg: {}",
                    site(&r["observer"]),
                    if key == "geometry" {
                        vector(&r["ecef_km"])
                    } else {
                        "[0.0; 3]".into()
                    },
                    if key == "iss_pipeline" {
                        format!("Some({})", time(&r["utc"]))
                    } else {
                        "None".into()
                    },
                    number(&r["range_km"]),
                    number(&r["azimuth_deg"]),
                    number(&r["elevation_deg"])
                ),
            };
            writeln!(source, "{i} => {ty} {{ {fields} }},").unwrap();
        }
        source.push_str("_ => panic!(\"reference index\"), } }\n");
    }
    for (key, value) in data["strict_less_than_tolerances"].as_object().unwrap() {
        writeln!(
            source,
            "const {}: f64 = {};",
            key.to_uppercase(),
            number(value)
        )
        .unwrap();
    }
    fs::write(
        PathBuf::from(env::var_os("OUT_DIR").unwrap()).join("numerical.rs"),
        source,
    )
    .unwrap();
}
