//! Host execution of the exact allocation-free future target checker.
use overhead_s3_benchmark::numerical::{self, Summary};

fn main() {
    let summary = numerical::run(|check, result| {
        println!("{:?} {} {result:?}", check.family, check.index);
    });
    println!("checked={} failed={}", summary.checked, summary.failed);
    assert_eq!(
        summary,
        Summary {
            checked: numerical::CHECK_COUNT,
            failed: 0
        }
    );
}
