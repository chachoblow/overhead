//! Embedded historical experiment inputs, not live data or accuracy references.
//! Provenance: tools/fixtures/README.md and core/tests/fixtures/README.md.
use overhead_core::{Satellite, sgp4::Elements};

pub struct HistoricalOrbit {
    pub name: &'static str,
    pub class: &'static str,
    pub satellite: Satellite,
}

pub fn historical_orbits() -> Result<Vec<HistoricalOrbit>, String> {
    let elements: Vec<Elements> =
        serde_json::from_str(include_str!("../../core/tests/fixtures/iss-25544.json"))
            .map_err(|e| e.to_string())?;
    if elements.len() != 1 {
        return Err("expected exactly one ISS fixture record".into());
    }
    let mut result = vec![HistoricalOrbit {
        name: "ISS",
        class: "LEO",
        satellite: Satellite::from_elements(&elements[0]).map_err(|e| e.to_string())?,
    }];
    let lines: Vec<_> = include_str!("../fixtures/pass-intervals.tle")
        .lines()
        .collect();
    let (records, remainder) = lines.as_chunks::<3>();
    if records.len() != 3 || !remainder.is_empty() {
        return Err("expected three named TLE records in the interval fixture".into());
    }
    for (lines, class) in records.iter().zip(["resonant-HEO", "GEO", "GNSS"]) {
        let elements = Elements::from_tle(None, lines[1].as_bytes(), lines[2].as_bytes())
            .map_err(|e| e.to_string())?;
        result.push(HistoricalOrbit {
            name: lines[0],
            class,
            satellite: Satellite::from_elements(&elements).map_err(|e| e.to_string())?,
        });
    }
    Ok(result)
}
