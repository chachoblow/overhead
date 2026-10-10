//! Deterministic offline case documents. No historical source bytes are edited.
use overhead_core::sgp4::{Elements, chrono::TimeDelta};
use serde::Serialize;
use serde_json::{Value, value::RawValue};

#[derive(Serialize)]
pub struct Document {
    pub key: String,
    pub text: String,
}

pub fn documents() -> Vec<Document> {
    let lines: Vec<_> = include_str!("../../../fixtures/catalogue-costs.tle")
        .lines()
        .collect();
    let (records, remainder) = lines.as_chunks::<3>();
    assert!(remainder.is_empty());
    let historical: Vec<_> = records
        .iter()
        .map(|r| Elements::from_tle(Some(r[0].into()), r[1].as_bytes(), r[2].as_bytes()).unwrap())
        .collect();
    assert_eq!(
        historical.iter().map(|e| e.norad_id).collect::<Vec<_>>(),
        [6251, 8195, 28129, 24208, 28057, 9880, 14128, 28626]
    );
    let base = serde_json::to_string(&historical).unwrap();
    let pool: Vec<&RawValue> =
        serde_json::from_str(include_str!("../../../fixtures/m2-pool/pool.json")).unwrap();
    let subset = |indices: Vec<usize>| {
        format!(
            "[\n{}\n]\n",
            indices
                .iter()
                .map(|&i| pool[i].get())
                .collect::<Vec<_>>()
                .join(",\n")
        )
    };
    let mut docs = vec![Document {
        key: "historical".into(),
        text: base,
    }];
    for size in [5, 9, 12, 16] {
        docs.push(Document {
            key: format!("mixed-{size}"),
            text: subset((0..size).collect()),
        });
    }
    for (offset, key) in [(0, "leo-heavy"), (1, "deep-space-heavy")] {
        docs.push(Document {
            key: key.into(),
            text: subset(
                (offset..16)
                    .step_by(2)
                    .chain((1 - offset..4).step_by(2))
                    .collect(),
            ),
        });
    }
    // Serialize typed elements, not a Value round trip: preserve the baseline's
    // floating-point serialization. Only the named synthetic fields change.
    let mut older = historical[0].clone();
    older.datetime -= TimeDelta::days(1);
    let mut conflict = historical[0].clone();
    conflict.mean_anomaly = (conflict.mean_anomaly + 1.) % 360.;
    docs.push(Document {
        key: "conflict".into(),
        text: serde_json::to_string(&[older, conflict]).unwrap(),
    });
    let original = serde_json::to_string(&historical[0]).unwrap();
    let wrong_center = original.replacen('{', "{\"CENTER_NAME\":\"MARS\",", 1);
    let mut invalid = historical[0].clone();
    invalid.eccentricity = 1.;
    docs.push(Document {
        key: "invalid".into(),
        text: format!(
            "[{wrong_center},{}]",
            serde_json::to_string(&invalid).unwrap()
        ),
    });
    docs.push(Document {
        key: "malformed".into(),
        text: "[".into(),
    });
    docs
}

pub fn check_pinned(docs: &[Document]) -> Result<Value, String> {
    let pinned: Value =
        serde_json::from_str(include_str!("../../../fixtures/m2-cases/inputs.json"))
            .map_err(|e| e.to_string())?;
    let rows = pinned["documents"]
        .as_array()
        .ok_or("missing pinned inputs")?;
    if rows.len() != docs.len() {
        return Err("pinned input count changed".into());
    }
    for (row, doc) in rows.iter().zip(docs) {
        if row["key"] != doc.key || row["text"] != doc.text || row["bytes"] != doc.text.len() {
            return Err(format!(
                "pinned input changed: {}; review, do not silently refresh",
                doc.key
            ));
        }
    }
    Ok(pinned)
}
