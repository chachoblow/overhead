//! Same-model work-count manifest, never target memory/numeric evidence.
use overhead_s3_benchmark::{
    catalogue_memory::{self as experiment, Case},
    suites::sample_count,
};

fn main() {
    let args: Vec<_> = std::env::args().skip(1).collect();
    assert_eq!(args.len(), 1, "usage: catalogue_expected SAMPLES");
    let samples = sample_count(&args[0]).expect("SAMPLES must be 1..5");
    println!("OVERHEAD_S3_CATALOGUE_EXPECTED_V1");
    println!("MATRIX,{samples},{}", experiment::CASE_COUNT);
    for id in 0..experiment::CASE_COUNT {
        let case = Case::at(id).unwrap();
        let catalogue = experiment::build(case.json()).unwrap();
        experiment::validate_catalogue(&catalogue, case);
        let (result, candidates) =
            experiment::aggregate(&catalogue, &case.config(), case.allowance);
        let work = experiment::work(&result, &candidates);
        println!(
            "CASE,{id},{},{},{},{}",
            case.size,
            case.window_s,
            case.allowance,
            case.json().len()
        );
        println!(
            "WORK,{id},{},{},{},{},{}",
            work.evaluations,
            work.passes,
            work.candidates,
            work.searched,
            u8::from(work.complete)
        );
    }
    println!("OVERHEAD_S3_EXPECTED_DONE");
}
