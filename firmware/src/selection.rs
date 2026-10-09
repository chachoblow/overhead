//! Shared build/host/target experiment selection; not device operating policy.
pub const AGE_DAYS: [i64; 7] = [-30, -7, -1, 0, 1, 7, 30];
pub const SUITE_NAMES: [&str; 11] = [
    "baseline",
    "scaling-16",
    "scaling-64",
    "ages-leo",
    "ages-heo",
    "ages-geo",
    "ages-gnss",
    "search-leo",
    "search-heo",
    "search-geo",
    "search-gnss",
];

pub fn sample_count(text: &str) -> Option<usize> {
    text.parse().ok().filter(|n| (1..=5).contains(n))
}
