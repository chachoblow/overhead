//! Offline catalogue assembly. Files/configuration/reporting live here;
//! validation and orbital conflict resolution live in overhead-core.
use std::{collections::BTreeSet, fmt::Write, fs, path::Path, path::PathBuf};

use overhead_core::{
    OmmElements,
    catalogue::{Catalogue, CatalogueBuilder, CatalogueDiagnostic, RecordOrigin},
    sgp4::chrono::NaiveDateTime,
    validate_utc_time,
};
use serde::Deserialize;
use serde_json::value::RawValue;

pub const USAGE: &str = "Usage: overhead-catalogue MANIFEST.json

Load named local OMM groups, merge by NORAD ID, and report selected elements.
Paths are relative to the manifest, not the working directory.
No network, wall clock, propagation, or implicit freshness cutoff.

Manifest:
  {\"groups\": [{\"name\": \"stations\", \"path\": \"stations.json\",
    \"source_url\": \"https://celestrak.org/NORAD/elements/gp.php?GROUP=stations&FORMAT=JSON\",
    \"fetched_at\": \"2026-10-06T08:00:00Z\"}]}
source_url and fetched_at are optional; missing/null means unknown.
fetched_at: YYYY-MM-DDTHH:MM:SS[.fraction]Z, 1957–2100; no leap seconds.

Invalid records and newest-epoch conflicts are skipped with stderr diagnostics.
Unreadable files, malformed documents/configuration, or an empty result fail
with exit 1 and no partial stdout report. Success/help exit 0.

Example (historical fixture, not live tracking data):
  cargo run -p overhead-tools --bin overhead-catalogue -- tools/examples/catalogue.json";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    groups: Vec<Group>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Group {
    name: String,
    path: PathBuf,
    source_url: Option<String>,
    fetched_at: Option<String>,
}

impl Manifest {
    fn validate(&self) -> Result<(), String> {
        if self.groups.is_empty() {
            return Err("manifest must configure at least one group".into());
        }
        let mut names = BTreeSet::new();
        for group in &self.groups {
            if group.name.trim().is_empty() || group.path.as_os_str().is_empty() {
                return Err("group name and path must be nonempty".into());
            }
            if !names.insert(&group.name) {
                return Err(format!("duplicate group name {:?}", group.name));
            }
            if group
                .source_url
                .as_ref()
                .is_some_and(|url| url.trim().is_empty())
            {
                return Err(format!("empty source_url for group {:?}", group.name));
            }
            if let Some(text) = &group.fetched_at {
                let parse = || -> Result<(), String> {
                    let utc = text.strip_suffix('Z').ok_or("UTC must end in Z")?;
                    let time = NaiveDateTime::parse_from_str(utc, "%Y-%m-%dT%H:%M:%S%.f")
                        .map_err(|error| error.to_string())?;
                    validate_utc_time(time).map_err(|error| error.to_string())
                };
                parse().map_err(|error| {
                    format!("invalid fetched_at for group {:?}: {error}", group.name)
                })?;
            }
        }
        Ok(())
    }

    fn origin(&self, origin: RecordOrigin) -> String {
        let group = &self.groups[origin.group];
        format!(
            "group {:?}, {} record {}",
            group.name,
            group.path.display(),
            origin.record + 1
        )
    }
}

fn diagnostics_report(
    manifest: &Manifest,
    parse_errors: &[(RecordOrigin, String)],
    diagnostics: &[CatalogueDiagnostic],
) -> String {
    let mut output = String::new();
    for (origin, error) in parse_errors {
        writeln!(
            output,
            "warning: skipped {}: invalid OMM record: {error}",
            manifest.origin(*origin)
        )
        .unwrap();
    }
    for diagnostic in diagnostics {
        match diagnostic {
            CatalogueDiagnostic::InvalidRecord {
                origin,
                norad_id,
                error,
            } => {
                writeln!(
                    output,
                    "warning: skipped NORAD {norad_id} at {}: {error}",
                    manifest.origin(*origin)
                )
                .unwrap();
            }
            CatalogueDiagnostic::EpochConflict {
                norad_id,
                epoch,
                origins,
            } => {
                writeln!(output, "warning: omitted NORAD {norad_id}: conflicting orbital values at newest valid epoch {}T{}Z", epoch.date(), epoch.time()).unwrap();
                for origin in origins {
                    writeln!(output, "  {}", manifest.origin(*origin)).unwrap();
                }
            }
        }
    }
    output
}

/// Fully loaded accepted catalogue with provenance report and ingestion warnings.
pub struct LoadedCatalogue {
    pub catalogue: Catalogue,
    pub report: String,
    pub diagnostics: String,
}

pub fn load(path: &Path) -> Result<LoadedCatalogue, String> {
    let data = fs::read_to_string(path)
        .map_err(|error| format!("cannot read manifest {}: {error}", path.display()))?;
    let manifest: Manifest = serde_json::from_str(&data)
        .map_err(|error| format!("invalid manifest {}: {error}", path.display()))?;
    manifest.validate()?;
    let directory = path.parent().unwrap_or(Path::new("."));
    let mut builder = CatalogueBuilder::new();
    let mut parse_errors = Vec::new();
    for (group_index, group) in manifest.groups.iter().enumerate() {
        let group_path = directory.join(&group.path);
        let data = fs::read_to_string(&group_path).map_err(|error| {
            format!(
                "cannot read group {:?} at {}: {error}",
                group.name,
                group_path.display()
            )
        })?;
        // Validate the whole array's syntax before interpreting records. Using
        // Value here would erase duplicate keys before checked OMM sees them.
        let records: Vec<&RawValue> = serde_json::from_str(&data).map_err(|error| {
            format!(
                "invalid OMM JSON document for group {:?} at {}: {error}",
                group.name,
                group_path.display()
            )
        })?;
        for (record, raw) in records.into_iter().enumerate() {
            let origin = RecordOrigin {
                group: group_index,
                record,
            };
            match serde_json::from_str::<OmmElements>(raw.get()) {
                Ok(elements) => builder.push(origin, elements),
                Err(error) => parse_errors.push((origin, error.to_string())),
            }
        }
    }
    let catalogue = builder.finish().map_err(|error| {
        let diagnostics = diagnostics_report(&manifest, &parse_errors, &error.diagnostics);
        format!("{error}\n{diagnostics}")
    })?;
    let diagnostics = diagnostics_report(&manifest, &parse_errors, catalogue.diagnostics());

    // Only publish a complete, nonempty catalogue report after all groups load.
    let mut output = String::new();
    writeln!(
        output,
        "Catalogue: {} satellites",
        catalogue.entries().len()
    )
    .unwrap();
    writeln!(output, "Configured groups: {}", manifest.groups.len()).unwrap();
    for entry in catalogue.entries() {
        let elements = entry.elements();
        let source = &manifest.groups[entry.selected_from().group];
        writeln!(output, "\nNORAD ID: {}", elements.norad_id).unwrap();
        writeln!(
            output,
            "Satellite: {}",
            elements.object_name.as_deref().unwrap_or("(unnamed)")
        )
        .unwrap();
        let names: Vec<_> = entry
            .groups()
            .iter()
            .map(|&index| manifest.groups[index].name.as_str())
            .collect();
        writeln!(output, "Groups: {}", names.join(", ")).unwrap();
        writeln!(
            output,
            "Element epoch (UTC): {}T{}Z",
            elements.datetime.date(),
            elements.datetime.time()
        )
        .unwrap();
        writeln!(
            output,
            "Selected from: {}",
            manifest.origin(entry.selected_from())
        )
        .unwrap();
        writeln!(
            output,
            "Source URL: {}",
            source.source_url.as_deref().unwrap_or("unknown")
        )
        .unwrap();
        writeln!(
            output,
            "Fetched at (UTC): {}",
            source.fetched_at.as_deref().unwrap_or("unknown")
        )
        .unwrap();
    }
    writeln!(
        output,
        "\nSelection: newest valid element epoch; equivalent ties use manifest/record order."
    )
    .unwrap();
    writeln!(
        output,
        "Fetch time is provenance, not element epoch or a freshness guarantee."
    )
    .unwrap();
    Ok(LoadedCatalogue {
        catalogue,
        report: output,
        diagnostics,
    })
}
