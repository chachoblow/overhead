use serde_json::Value;
use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_overhead-m2-preflight"))
}

#[test]
fn cli_and_embedded_inputs() {
    for help in ["--help", "-h"] {
        let out = command().arg(help).output().unwrap();
        assert!(out.status.success());
        assert!(out.stderr.is_empty());
        assert!(String::from_utf8(out.stdout).unwrap().contains("Usage:"));
    }
    for args in [
        vec!["--refresh"],
        vec!["--help", "extra"],
        vec!["--inputs", "extra"],
        vec!["file.json"],
    ] {
        let out = command().args(args).output().unwrap();
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
    let out = command()
        .arg("--inputs")
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(out.status.success());
    let docs: Value = serde_json::from_slice(&out.stdout).unwrap();
    let pinned: Value =
        serde_json::from_str(include_str!("../fixtures/m2-cases/inputs.json")).unwrap();
    for (doc, pin) in docs
        .as_array()
        .unwrap()
        .iter()
        .zip(pinned["documents"].as_array().unwrap())
    {
        assert_eq!(doc["key"], pin["key"]);
        assert_eq!(doc["text"], pin["text"]);
    }
    assert_eq!(docs.as_array().unwrap().len(), 10);
}

#[test]
fn all_cases_repeat_release_and_preserve_controls() {
    // Allocator observations must run in an isolated, single-threaded CLI, never
    // inside the concurrently allocating Rust test harness.
    let out = command()
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(out.stderr.is_empty());
    let mut actual: Value = serde_json::from_slice(&out.stdout).unwrap();
    let mut expected: Value = serde_json::from_str(include_str!(
        "../../docs/evaluations/m2-host-preflight.json"
    ))
    .unwrap();
    assert_eq!(actual["cases"].as_array().unwrap().len(), 20);
    for (index, row) in actual["cases"].as_array().unwrap().iter().enumerate() {
        assert_eq!(row["case"]["id"], index + 1);
        assert!(row["input_json_bytes"].as_u64().unwrap() <= 32768);
        assert!(row["input_records"].as_u64().unwrap() <= 32);
        let memory = &row["requested_memory"];
        assert_eq!(memory["destruction"]["after"]["live"], 0);
        assert_eq!(memory["destruction"]["after"]["failures"], 0);
        let peak = memory["pipeline_peak"].as_u64().unwrap();
        for phase in [
            "input_copy",
            "initialization",
            "input_release",
            "aggregation",
            "destruction",
        ] {
            if memory[phase].is_null() {
                assert_eq!(index, 15);
                assert_eq!(phase, "aggregation");
            } else {
                assert!(peak >= memory[phase]["peak"].as_u64().unwrap());
            }
        }
    }
    let rows = actual["cases"].as_array().unwrap();
    for (dropped, retained, control) in [(16, 17, 1), (18, 19, 6)] {
        let input = rows[dropped]["input_json_bytes"].as_u64().unwrap();
        for row in [&rows[dropped], &rows[retained]] {
            assert_eq!(row["accepted"], rows[control]["accepted"]);
            assert_eq!(row["diagnostics"], rows[control]["diagnostics"]);
            assert_eq!(row["search"], rows[control]["search"]);
            assert_eq!(
                row["requested_memory"]["input_copy"]["after"]["live"],
                input
            );
            assert_eq!(
                row["requested_memory"]["initialization"]["peak"]
                    .as_u64()
                    .unwrap(),
                rows[control]["requested_memory"]["initialization"]["peak"]
                    .as_u64()
                    .unwrap()
                    + input
            );
        }
        let control_live = rows[control]["requested_memory"]["aggregation"]["after"]["live"]
            .as_u64()
            .unwrap();
        assert_eq!(
            rows[dropped]["requested_memory"]["aggregation"]["after"]["live"],
            control_live
        );
        assert_eq!(
            rows[retained]["requested_memory"]["aggregation"]["after"]["live"],
            control_live + input
        );
    }
    // Work/diagnostics/input bytes are portable expectations. Capacity/size
    // observations are toolchain/host-specific evidence, NOT a core API contract.
    if actual["host_os"] != expected["host_os"]
        || actual["host_arch"] != expected["host_arch"]
        || actual["pointer_bits"] != expected["pointer_bits"]
    {
        for report in [&mut actual, &mut expected] {
            for key in ["host_os", "host_arch", "pointer_bits"] {
                report.as_object_mut().unwrap().remove(key);
            }
            for row in report["cases"].as_array_mut().unwrap() {
                row.as_object_mut().unwrap().remove("requested_memory");
            }
        }
    }
    assert_eq!(
        actual, expected,
        "changed evidence requires preflight review, not a silent refresh"
    );
}
