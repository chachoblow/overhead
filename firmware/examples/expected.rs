//! Host-only expected-work manifest. Does not access hardware or measure time.
use overhead_s3_benchmark::{
    satellites,
    suites::{Suite, sample_count},
};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    let [name, samples] = args.as_slice() else {
        eprintln!(
            "Usage: expected SUITE SAMPLES\nSuites: {:?}; samples: 1..5",
            overhead_s3_benchmark::suites::SUITE_NAMES
        );
        std::process::exit(1);
    };
    let Some((suite, samples)) = Suite::parse(name).zip(sample_count(samples)) else {
        eprintln!("Invalid suite or sample count");
        std::process::exit(1);
    };
    // Buffer so a propagation/search failure cannot publish a partial manifest.
    use std::fmt::Write;
    let mut text = String::from("OVERHEAD_S3_EXPECTED_V2\n");
    writeln!(text, "SUITE,{},{samples},{}", suite.name(), suite.len()).unwrap();
    let satellites = satellites();
    for (id, workload) in suite.workloads().enumerate() {
        let work = workload.prepare().run(&satellites);
        writeln!(text, "CASE,{id},{workload}").unwrap();
        writeln!(
            text,
            "WORK,{id},{},{},{}",
            work.state_evaluations, work.searches, work.passes
        )
        .unwrap();
    }
    text.push_str("OVERHEAD_S3_EXPECTED_DONE\n");
    print!("{text}");
}
