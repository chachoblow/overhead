//! OMM deserialization that checks physical metadata before sgp4 discards it.

use alloc::{format, string::String};
use serde::{Deserialize, Deserializer};

/// Parsed elements with compatible Earth/TEME/UTC/SGP4 metadata.
///
/// Deserialize JSON objects (or arrays of these) through this adapter rather
/// than directly into [`sgp4::Elements`], which ignores these metadata fields.
/// Omitted fields use CelesTrak GP defaults; explicit values must match exactly.
/// Nulls and non-string values are rejected. Unrelated extra fields are ignored.
///
/// This checks metadata, not orbital validity: pass [`Self::elements`] to
/// [`crate::Satellite::from_elements`] for numeric and epoch validation.
/// Available with the `omm` feature; requires alloc, not std or file I/O.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(try_from = "RawOmmElements")]
pub struct OmmElements(sgp4::Elements);

impl OmmElements {
    /// Borrows the parsed elements, including identity and display metadata.
    pub fn elements(&self) -> &sgp4::Elements {
        &self.0
    }

    /// Returns the parsed elements after physical metadata has been checked.
    pub fn into_elements(self) -> sgp4::Elements {
        self.0
    }
}

#[derive(Deserialize)]
struct RawOmmElements {
    #[serde(flatten)]
    elements: sgp4::Elements,
    #[serde(rename = "CENTER_NAME", default, deserialize_with = "present_string")]
    center_name: Option<String>,
    #[serde(rename = "REF_FRAME", default, deserialize_with = "present_string")]
    ref_frame: Option<String>,
    #[serde(rename = "TIME_SYSTEM", default, deserialize_with = "present_string")]
    time_system: Option<String>,
    #[serde(
        rename = "MEAN_ELEMENT_THEORY",
        default,
        deserialize_with = "present_string"
    )]
    mean_element_theory: Option<String>,
}

// Only omission selects a default. Option's usual deserializer would also
// accept an explicit null, hiding malformed metadata as an omitted field.
fn present_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

impl TryFrom<RawOmmElements> for OmmElements {
    type Error = String;

    fn try_from(raw: RawOmmElements) -> Result<Self, Self::Error> {
        for (field, value, expected) in [
            ("CENTER_NAME", raw.center_name, "EARTH"),
            ("REF_FRAME", raw.ref_frame, "TEME"),
            ("TIME_SYSTEM", raw.time_system, "UTC"),
            ("MEAN_ELEMENT_THEORY", raw.mean_element_theory, "SGP4"),
        ] {
            if let Some(value) = value
                && value != expected
            {
                return Err(format!(
                    "unsupported OMM {field} {value:?}; expected {expected}"
                ));
            }
        }
        Ok(Self(raw.elements))
    }
}
