use std::process::Command;

fn command() -> Command {
    Command::new(env!("CARGO_BIN_EXE_overhead-m2-pool-preflight"))
}

#[test]
fn help_and_invalid_arguments() {
    for help in ["--help", "-h"] {
        let output = command().arg(help).output().unwrap();
        assert!(output.status.success());
        assert!(output.stderr.is_empty());
        assert!(String::from_utf8(output.stdout).unwrap().contains("Usage:"));
    }
    for args in [
        vec!["--refresh"],
        vec!["--help", "extra"],
        vec!["input.json"],
    ] {
        let output = command().args(args).output().unwrap();
        assert_eq!(output.status.code(), Some(1));
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[test]
fn embedded_inputs_work_outside_repository() {
    let output = command()
        .current_dir(std::env::temp_dir())
        .output()
        .unwrap();
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stderr.is_empty());
    let actual: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let expected: serde_json::Value = serde_json::from_str(include_str!(
        "../../docs/evaluations/m2-pool-suitability.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
}
