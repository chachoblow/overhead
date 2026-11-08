//! 0071 slice 1: the fixed CelesTrak element request (since 0077 the `visual`
//! group, previously STATIONS), a bounded HTTP/1.0
//! response parser and catalogue ingestion of the body. Host-tested and
//! transport-free: no socket, TLS, clock, retry or persistence lives here.
//! Every error is final for the boot (0071: no automatic retry).
use alloc::{string::String, vec::Vec};
use overhead_core::{
    OmmElements,
    catalogue::{Catalogue, CatalogueBuilder, CatalogueDiagnostic, RecordOrigin},
    sgp4::chrono::NaiveDateTime,
};
use serde_json::value::RawValue;

pub const HOST: &str = "celestrak.org";
pub const PORT: u16 = 443;
pub const PATH: &str = "/NORAD/elements/gp.php?GROUP=visual&FORMAT=JSON";

/// One GET per connection. HTTP/1.0 rules out chunked transfer coding, so a
/// complete response is the head, `Content-Length` body bytes and a close.
pub const REQUEST: &str = concat!(
    "GET /NORAD/elements/gp.php?GROUP=visual&FORMAT=JSON HTTP/1.0\r\n",
    "Host: celestrak.org\r\n",
    "User-Agent: overhead-s3/0.1 (+https://github.com/chachoblow/overhead)\r\n",
    "Accept: application/json\r\n",
    "Connection: close\r\n",
    "\r\n",
);

/// Largest accepted status line plus headers, including the blank line.
/// CelesTrak's measured head is about 230 bytes.
pub const MAX_HEAD: usize = 2 * 1024;
/// Largest accepted body (0077): about twice the measured 64,659-byte
/// `visual` array of 156 records, room for ~310 records of ~415 bytes.
pub const MAX_BODY: usize = 128 * 1024;
/// Receive buffer that holds any acceptable complete response.
pub const MAX_RESPONSE: usize = MAX_HEAD + MAX_BODY;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Head {
    pub status: u16,
    pub content_length: usize,
    /// Offset of the first body byte in the response.
    pub body_start: usize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HttpError {
    /// No blank line within `MAX_HEAD` bytes.
    HeadTooLarge,
    /// Connection closed before the head was complete.
    IncompleteHead,
    MalformedStatusLine,
    MalformedHeader,
    /// Any status other than 200, including 301, 403 and 404. Never retried:
    /// CelesTrak firewalls addresses after 50 such errors in 2 hours.
    Status(u16),
    MissingContentLength,
    /// Repeated `Content-Length` headers, even if equal.
    DuplicateContentLength,
    BadContentLength,
    /// Any `Transfer-Encoding`; not valid in a reply to HTTP/1.0.
    TransferEncoding,
    BodyTooLarge {
        length: usize,
    },
    /// Connection closed before `Content-Length` bytes arrived.
    Truncated {
        expected: usize,
        received: usize,
    },
    /// More bytes than `Content-Length` arrived before close.
    Overrun {
        expected: usize,
        received: usize,
    },
}

/// Parses the response head once it is complete. `Ok(None)` means more bytes
/// are needed. Status is checked before length, so a 403 is reported as such
/// whatever its headers. A 200 needs a single `Content-Length` within the cap,
/// so the receiver can stop early instead of downloading an oversized body.
pub fn parse_head(bytes: &[u8]) -> Result<Option<Head>, HttpError> {
    parse_head_capped(bytes, MAX_BODY)
}

/// [`parse_head`] with another body cap (OTA manifests and images, 0085).
pub fn parse_head_capped(bytes: &[u8], max_body: usize) -> Result<Option<Head>, HttpError> {
    let Some(end) = find(bytes, b"\r\n\r\n") else {
        // A full-size buffer without the terminator cannot become valid.
        return if bytes.len() >= MAX_HEAD {
            Err(HttpError::HeadTooLarge)
        } else {
            Ok(None)
        };
    };
    let body_start = end + 4;
    if body_start > MAX_HEAD {
        return Err(HttpError::HeadTooLarge);
    }
    // Every head line, including the last, ends in CRLF; a bare LF is malformed.
    let mut lines = bytes[..end + 2]
        .split_inclusive(|&b| b == b'\n')
        .map(|line| line.strip_suffix(b"\r\n"));
    let status_line = lines.next().flatten();
    let status = status_code(status_line.ok_or(HttpError::MalformedStatusLine)?)?;
    if status != 200 {
        return Err(HttpError::Status(status));
    }
    let mut content_length = None;
    for line in lines {
        let (name, value) = header(line.ok_or(HttpError::MalformedHeader)?)?;
        if name.eq_ignore_ascii_case(b"content-length") {
            if content_length.is_some() {
                return Err(HttpError::DuplicateContentLength);
            }
            content_length = Some(decimal(value)?);
        } else if name.eq_ignore_ascii_case(b"transfer-encoding") {
            return Err(HttpError::TransferEncoding);
        }
    }
    let content_length = content_length.ok_or(HttpError::MissingContentLength)?;
    if content_length > max_body {
        return Err(HttpError::BodyTooLarge {
            length: content_length,
        });
    }
    Ok(Some(Head {
        status,
        content_length,
        body_start,
    }))
}

/// Checks a response received up to connection close and returns its body.
pub fn parse_response(bytes: &[u8]) -> Result<&[u8], HttpError> {
    parse_response_capped(bytes, MAX_BODY)
}

/// [`parse_response`] with another body cap.
pub fn parse_response_capped(bytes: &[u8], max_body: usize) -> Result<&[u8], HttpError> {
    let head = parse_head_capped(bytes, max_body)?.ok_or(HttpError::IncompleteHead)?;
    let body = &bytes[head.body_start..];
    let (expected, received) = (head.content_length, body.len());
    match received.cmp(&expected) {
        core::cmp::Ordering::Less => Err(HttpError::Truncated { expected, received }),
        core::cmp::Ordering::Greater => Err(HttpError::Overrun { expected, received }),
        core::cmp::Ordering::Equal => Ok(body),
    }
}

fn find(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}

// `HTTP/1.<digit> <3 digits>` followed by end of line or a space and reason.
fn status_code(line: &[u8]) -> Result<u16, HttpError> {
    let rest = line
        .strip_prefix(b"HTTP/1.")
        .ok_or(HttpError::MalformedStatusLine)?;
    match rest {
        [minor, b' ', a, b, c, tail @ ..]
            if minor.is_ascii_digit()
                && [a, b, c].iter().all(|d| d.is_ascii_digit())
                && (tail.is_empty() || tail[0] == b' ') =>
        {
            Ok([a, b, c]
                .iter()
                .fold(0, |code, &&d| code * 10 + u16::from(d - b'0')))
        }
        _ => Err(HttpError::MalformedStatusLine),
    }
}

fn header(line: &[u8]) -> Result<(&[u8], &[u8]), HttpError> {
    let colon = line
        .iter()
        .position(|&b| b == b':')
        .ok_or(HttpError::MalformedHeader)?;
    let name = &line[..colon];
    // Obsolete line folding and whitespace before the colon are rejected.
    if name.is_empty()
        || name
            .iter()
            .any(|b| b.is_ascii_whitespace() || b.is_ascii_control())
    {
        return Err(HttpError::MalformedHeader);
    }
    let value = line[colon + 1..].trim_ascii();
    if value.iter().any(|&b| b.is_ascii_control() && b != b'\t') {
        return Err(HttpError::MalformedHeader);
    }
    Ok((name, value))
}

fn decimal(value: &[u8]) -> Result<usize, HttpError> {
    if value.is_empty() || !value.iter().all(u8::is_ascii_digit) {
        return Err(HttpError::BadContentLength);
    }
    value.iter().try_fold(0usize, |n, &d| {
        n.checked_mul(10)
            .and_then(|n| n.checked_add(usize::from(d - b'0')))
            .ok_or(HttpError::BadContentLength)
    })
}

/// A record that failed OMM deserialization (JSON shape or metadata).
#[derive(Debug, PartialEq)]
pub struct RecordError {
    pub record: usize,
    pub message: String,
}

#[derive(Debug)]
pub enum IngestError {
    /// The body is not a JSON array; the whole download is rejected.
    Document(String),
    /// No record produced a usable satellite.
    Empty {
        records: usize,
        record_errors: Vec<RecordError>,
        diagnostics: Vec<CatalogueDiagnostic>,
    },
}

pub struct Ingested {
    pub catalogue: Catalogue,
    /// Array length, including records that were rejected.
    pub records: usize,
    pub record_errors: Vec<RecordError>,
}

/// Summary for the `live-fetch` diagnostic report.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Summary {
    pub records: usize,
    pub accepted: usize,
    pub record_errors: usize,
    pub invalid: usize,
    pub conflicts: usize,
    pub oldest_epoch: NaiveDateTime,
    pub newest_epoch: NaiveDateTime,
}

/// Builds a catalogue from one element body through the checked
/// `overhead-core` path, as group 0. A malformed array rejects the document;
/// a malformed record is reported and skipped, as in the M2 loader.
pub fn ingest(body: &[u8]) -> Result<Ingested, IngestError> {
    let raw: Vec<&RawValue> =
        serde_json::from_slice(body).map_err(|e| IngestError::Document(alloc::format!("{e}")))?;
    let records = raw.len();
    let mut builder = CatalogueBuilder::new();
    let mut record_errors = Vec::new();
    for (record, value) in raw.into_iter().enumerate() {
        match serde_json::from_str::<OmmElements>(value.get()) {
            Ok(elements) => builder.push(RecordOrigin { group: 0, record }, elements),
            Err(e) => record_errors.push(RecordError {
                record,
                message: alloc::format!("{e}"),
            }),
        }
    }
    match builder.finish() {
        Ok(catalogue) => Ok(Ingested {
            catalogue,
            records,
            record_errors,
        }),
        Err(empty) => Err(IngestError::Empty {
            records,
            record_errors,
            diagnostics: empty.diagnostics,
        }),
    }
}

impl Ingested {
    pub fn summary(&self) -> Summary {
        let entries = self.catalogue.entries();
        let epochs = entries.iter().map(|e| e.satellite().epoch());
        let count = |conflict: bool| {
            self.catalogue
                .diagnostics()
                .iter()
                .filter(|d| matches!(d, CatalogueDiagnostic::EpochConflict { .. }) == conflict)
                .count()
        };
        Summary {
            records: self.records,
            accepted: entries.len(),
            record_errors: self.record_errors.len(),
            invalid: count(false),
            conflicts: count(true),
            // A finished catalogue is never empty.
            oldest_epoch: epochs.clone().min().expect("nonempty catalogue"),
            newest_epoch: epochs.max().expect("nonempty catalogue"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use alloc::{format, vec};

    const FIXTURE: &[u8] = include_bytes!("../fixtures/celestrak-stations-20261101.json");
    // Response head as received with the fixture (2026-11-01 05:04:12 UTC).
    const HEAD: &str = "HTTP/1.1 200 OK\r\n\
        Content-Type: application/json; charset=UTF-8\r\n\
        Server: Microsoft-IIS/10.0\r\n\
        X-Powered-By: PHP/8.5.12\r\n\
        Content-Disposition: filename=\"stations.json\"\r\n\
        Date: Sun, 01 Nov 2026 05:04:12 GMT\r\n\
        Content-Length: 8441\r\n\r\n";

    fn response(head: &str, body: &[u8]) -> Vec<u8> {
        let mut bytes = Vec::from(head.as_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    fn ok(body: &[u8]) -> Vec<u8> {
        response(
            &format!("HTTP/1.1 200 OK\r\nContent-Length: {}\r\n\r\n", body.len()),
            body,
        )
    }

    fn epoch(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f").unwrap()
    }

    #[test]
    fn request_is_one_fixed_http10_get() {
        assert!(REQUEST.starts_with(&format!("GET {PATH} HTTP/1.0\r\n")));
        assert!(REQUEST.contains(&format!("\r\nHost: {HOST}\r\n")));
        assert!(REQUEST.contains("\r\nUser-Agent: overhead-s3/"));
        assert!(REQUEST.ends_with("\r\nConnection: close\r\n\r\n"));
        assert_eq!(REQUEST.matches("\r\n\r\n").count(), 1);
        assert!(REQUEST.is_ascii());
    }

    const VISUAL: &[u8] = include_bytes!("../fixtures/celestrak-visual-20261103.json");

    #[test]
    fn request_names_the_visual_group() {
        assert!(PATH.contains("GROUP=visual&FORMAT=JSON"));
        assert!(!REQUEST.contains("STATIONS"));
    }

    #[test]
    fn visual_fixture_fits_and_ingests_completely() {
        assert_eq!(VISUAL.len(), 64_659);
        assert!(VISUAL.ends_with(b"}]\r\n"));
        assert!(VISUAL.len() * 2 <= MAX_BODY + 2048);
        let bytes = ok(VISUAL);
        assert!(bytes.len() <= MAX_RESPONSE);
        assert_eq!(parse_response(&bytes), Ok(VISUAL));
        let ingested = ingest(VISUAL).unwrap();
        assert!(ingested.record_errors.is_empty());
        let ids: Vec<u64> = ingested
            .catalogue
            .entries()
            .iter()
            .map(|e| e.elements().norad_id)
            .collect();
        assert!(ids.contains(&25544) && ids.contains(&20580) && ids.contains(&48274));
        assert_eq!(
            ingested.summary(),
            Summary {
                records: 156,
                accepted: 156,
                record_errors: 0,
                invalid: 0,
                conflicts: 0,
                oldest_epoch: epoch("2026-11-01T15:41:22.462944"),
                newest_epoch: epoch("2026-11-02T22:18:20.337120"),
            }
        );
    }

    #[test]
    fn fixture_is_the_pinned_download() {
        assert_eq!(FIXTURE.len(), 8441);
        assert!(FIXTURE.ends_with(b"}]\r\n"));
        assert!(FIXTURE.len() <= MAX_BODY);
    }

    #[test]
    fn fixture_response_parses_and_ingests() {
        let bytes = response(HEAD, FIXTURE);
        assert!(bytes.len() <= MAX_RESPONSE);
        let head = parse_head(&bytes).unwrap().unwrap();
        assert_eq!((head.status, head.content_length), (200, 8441));
        assert_eq!(parse_response(&bytes), Ok(FIXTURE));

        let ingested = ingest(FIXTURE).unwrap();
        assert!(ingested.record_errors.is_empty());
        assert!(ingested.catalogue.diagnostics().is_empty());
        let ids: Vec<u64> = ingested
            .catalogue
            .entries()
            .iter()
            .map(|e| e.elements().norad_id)
            .collect();
        assert_eq!(ids.len(), 20);
        assert_eq!(ids[0], 25544);
        assert_eq!(
            ingested.summary(),
            Summary {
                records: 20,
                accepted: 20,
                record_errors: 0,
                invalid: 0,
                conflicts: 0,
                oldest_epoch: epoch("2026-10-31T06:35:49.011648"),
                newest_epoch: epoch("2026-10-31T22:51:14.800896"),
            }
        );
    }

    #[test]
    fn head_is_reported_only_when_complete_at_every_split() {
        let bytes = response(HEAD, FIXTURE);
        let complete = HEAD.len();
        for n in 0..=bytes.len() {
            let parsed = parse_head(&bytes[..n]).unwrap();
            assert_eq!(parsed.is_some(), n >= complete, "split {n}");
            if n < bytes.len() {
                assert!(parse_response(&bytes[..n]).is_err(), "split {n}");
            }
        }
    }

    #[test]
    fn non_200_statuses_are_typed_failures() {
        let policy = "GP data has not updated since your last successful download";
        for (head, code) in [
            (
                format!(
                    "HTTP/1.1 403 Forbidden\r\nContent-Length: {}\r\n\r\n",
                    policy.len()
                ),
                403,
            ),
            ("HTTP/1.1 404 Not Found\r\n\r\n".into(), 404),
            (
                "HTTP/1.1 301 Moved Permanently\r\nLocation: https://example.org/\r\n\r\n".into(),
                301,
            ),
            // Status wins over otherwise invalid headers.
            (
                "HTTP/1.0 500 Oops\r\nTransfer-Encoding: chunked\r\n\r\n".into(),
                500,
            ),
            ("HTTP/1.1 204\r\n\r\n".into(), 204),
        ] {
            let bytes = response(&head, policy.as_bytes());
            assert_eq!(parse_head(&bytes), Err(HttpError::Status(code)));
            assert_eq!(parse_response(&bytes), Err(HttpError::Status(code)));
        }
    }

    #[test]
    fn malformed_status_lines_are_rejected() {
        for line in [
            "HTTP/2 200 OK",
            "HTTP/1.1 20 OK",
            "HTTP/1.1 2000 OK",
            "HTTP/1.1  200 OK",
            "HTTP/1.x 200 OK",
            "ICY 200 OK",
            "http/1.1 200 OK",
            "",
        ] {
            let bytes = response(&format!("{line}\r\nContent-Length: 2\r\n\r\n"), b"[]");
            assert_eq!(
                parse_head(&bytes),
                Err(HttpError::MalformedStatusLine),
                "{line:?}"
            );
        }
    }

    #[test]
    fn content_length_rules() {
        let cases: [(&str, Result<Option<usize>, HttpError>); 12] = [
            ("content-length: 2", Ok(Some(2))),
            ("CONTENT-LENGTH:2  ", Ok(Some(2))),
            ("Content-Length: 131072", Ok(Some(MAX_BODY))),
            (
                "Content-Length: 131073",
                Err(HttpError::BodyTooLarge { length: 131073 }),
            ),
            (
                "Content-Length: 99999999999999999999999",
                Err(HttpError::BadContentLength),
            ),
            ("Content-Length: ", Err(HttpError::BadContentLength)),
            ("Content-Length: -1", Err(HttpError::BadContentLength)),
            ("Content-Length: 1 2", Err(HttpError::BadContentLength)),
            ("Content-Length: 0x10", Err(HttpError::BadContentLength)),
            (
                "Content-Length: 2\r\nContent-Length: 2",
                Err(HttpError::DuplicateContentLength),
            ),
            (
                "Transfer-Encoding: chunked",
                Err(HttpError::TransferEncoding),
            ),
            ("X-Other: 1", Err(HttpError::MissingContentLength)),
        ];
        for (header, expected) in cases {
            let bytes = response(&format!("HTTP/1.1 200 OK\r\n{header}\r\n\r\n"), b"");
            let got = parse_head(&bytes).map(|h| h.map(|h| h.content_length));
            assert_eq!(got, expected, "{header:?}");
        }
    }

    #[test]
    fn malformed_headers_are_rejected() {
        for header in [
            "No colon here",
            ": empty name",
            "Bad Name: 1",
            " folded: 1",
            "Lf-only: 1\nX: 2",
            "Cr-inside: a\rb",
        ] {
            let bytes = response(
                &format!("HTTP/1.1 200 OK\r\n{header}\r\nContent-Length: 0\r\n\r\n"),
                b"",
            );
            assert_eq!(
                parse_head(&bytes),
                Err(HttpError::MalformedHeader),
                "{header:?}"
            );
        }
    }

    #[test]
    fn oversized_or_unterminated_heads_fail() {
        let filler = format!("X-Filler: {}", "a".repeat(MAX_HEAD));
        let bytes = response(
            &format!("HTTP/1.1 200 OK\r\n{filler}\r\nContent-Length: 2\r\n\r\n"),
            b"[]",
        );
        assert_eq!(parse_head(&bytes[..MAX_HEAD - 1]), Ok(None));
        assert_eq!(parse_head(&bytes[..MAX_HEAD]), Err(HttpError::HeadTooLarge));
        assert_eq!(parse_head(&bytes), Err(HttpError::HeadTooLarge));
        assert_eq!(
            parse_response(b"HTTP/1.1 200 OK\r\n"),
            Err(HttpError::IncompleteHead)
        );
        assert_eq!(parse_response(b""), Err(HttpError::IncompleteHead));
    }

    #[test]
    fn body_must_match_content_length_exactly() {
        let mut bytes = response(HEAD, FIXTURE);
        bytes.push(b'\n');
        assert_eq!(
            parse_response(&bytes),
            Err(HttpError::Overrun {
                expected: 8441,
                received: 8442
            })
        );
        bytes.truncate(HEAD.len() + 100);
        assert_eq!(
            parse_response(&bytes),
            Err(HttpError::Truncated {
                expected: 8441,
                received: 100
            })
        );
        assert_eq!(parse_response(&ok(b"")), Ok(&b""[..]));
    }

    #[test]
    fn non_array_bodies_reject_the_document() {
        for body in [
            &b"GP data has not updated since your last successful download"[..],
            b"{\"OBJECT_NAME\":\"ISS\"}",
            b"[",
            &FIXTURE[..FIXTURE.len() - 10],
            b"[{}] trailing",
            b"\xff[]",
        ] {
            assert!(
                matches!(ingest(body), Err(IngestError::Document(_))),
                "{:?}",
                core::str::from_utf8(body)
            );
        }
    }

    // Rewrites one record of the real fixture through JSON values.
    fn with_record(index: usize, edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
        let mut records: Vec<serde_json::Value> = serde_json::from_slice(FIXTURE).unwrap();
        edit(&mut records[index]);
        serde_json::to_vec(&records).unwrap()
    }

    #[test]
    fn bad_records_are_reported_and_the_rest_accepted() {
        // Metadata the checked adapter rejects: a parse error.
        let body = with_record(3, |r| r["REF_FRAME"] = "GCRF".into());
        let ingested = ingest(&body).unwrap();
        assert_eq!(ingested.record_errors.len(), 1);
        assert_eq!(ingested.record_errors[0].record, 3);
        assert!(ingested.record_errors[0].message.contains("REF_FRAME"));
        assert_eq!(ingested.summary().accepted, 19);

        // Wrong field type: also a parse error.
        let body = with_record(0, |r| r["MEAN_MOTION"] = "fast".into());
        assert_eq!(ingest(&body).unwrap().record_errors[0].record, 0);

        // Parses but fails numeric validation: a catalogue diagnostic.
        let body = with_record(5, |r| r["ECCENTRICITY"] = 1.5.into());
        let summary = ingest(&body).unwrap().summary();
        assert_eq!(
            (summary.accepted, summary.invalid, summary.record_errors),
            (19, 1, 0)
        );

        // A duplicate ID at the same epoch with a different orbit conflicts.
        let mut records: Vec<serde_json::Value> = serde_json::from_slice(FIXTURE).unwrap();
        let mut twin = records[0].clone();
        twin["INCLINATION"] = 10.0.into();
        records.push(twin);
        let summary = ingest(&serde_json::to_vec(&records).unwrap())
            .unwrap()
            .summary();
        assert_eq!(
            (summary.records, summary.accepted, summary.conflicts),
            (21, 19, 1)
        );
    }

    #[test]
    fn no_usable_record_is_an_empty_failure() {
        assert!(matches!(
            ingest(b"[]"),
            Err(IngestError::Empty { records: 0, .. })
        ));
        let body = serde_json::to_vec(&vec![serde_json::json!({ "OBJECT_NAME": "X" })]).unwrap();
        match ingest(&body) {
            Err(IngestError::Empty {
                records: 1,
                record_errors,
                diagnostics,
            }) => {
                assert_eq!(record_errors.len(), 1);
                assert!(diagnostics.is_empty());
            }
            _ => panic!("expected empty catalogue"),
        }
    }
}
