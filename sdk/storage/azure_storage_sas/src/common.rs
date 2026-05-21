// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::{credentials::Secret, hmac::hmac_sha256, Result};
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use time::{macros::format_description, OffsetDateTime};

/// The SAS time format with second precision: `yyyy-MM-ddTHH:mm:ssZ`.
const SAS_TIME_FORMAT: &[time::format_description::FormatItem<'static>] =
    format_description!("[year]-[month]-[day]T[hour]:[minute]:[second]Z");

/// Format a UTC `OffsetDateTime` using the canonical SAS time format.
///
/// The input is converted to UTC. Sub-second precision is dropped.
pub(crate) fn format_sas_time(value: OffsetDateTime) -> String {
    value
        .to_offset(time::UtcOffset::UTC)
        .replace_millisecond(0)
        .unwrap_or(value)
        .format(SAS_TIME_FORMAT)
        .expect("SAS time format is infallible")
}

/// Percent-encoding set matching the .NET storage SAS implementation
/// (`Uri.EscapeDataString`-equivalent for path segments). Encodes everything
/// except unreserved characters.
const PATH_SEGMENT: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'/')
    .add(b'<')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'\\')
    .add(b'^')
    .add(b'|')
    .add(b'+')
    .add(b'&')
    .add(b'=')
    .add(b';')
    .add(b':')
    .add(b'@')
    .add(b'$')
    .add(b',');

/// Percent-encode a single path segment for inclusion in a SAS URL.
pub(crate) fn encode_segment(segment: &str) -> String {
    utf8_percent_encode(segment, PATH_SEGMENT).to_string()
}

/// Append a path component (one or more `/`-separated segments) to the given
/// URL, preserving an existing trailing slash on the URL and percent-encoding
/// each segment.
pub(crate) fn append_path(endpoint: &str, path: &str) -> String {
    let trimmed_endpoint = endpoint.trim_end_matches('/');
    let trimmed_path = path.trim_matches('/');
    if trimmed_path.is_empty() {
        return trimmed_endpoint.to_string();
    }
    let encoded: Vec<String> = trimmed_path.split('/').map(encode_segment).collect();
    format!("{}/{}", trimmed_endpoint, encoded.join("/"))
}

/// Compute the base64-encoded HMAC-SHA256 signature of `string_to_sign` using
/// the base64-encoded `key`.
pub(crate) fn sign(string_to_sign: &str, key: &str) -> Result<String> {
    hmac_sha256(string_to_sign, &Secret::new(key.to_string()))
}

/// Percent-encoding set for SAS query parameter values
/// (form-urlencoded-compatible).
const QUERY: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b'/')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'`')
    .add(b'{')
    .add(b'}')
    .add(b'\\')
    .add(b'^')
    .add(b'|');

/// Percent-encode a single SAS query parameter value.
pub(crate) fn encode_query(value: &str) -> String {
    utf8_percent_encode(value, QUERY).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    #[test]
    fn formats_sas_time_to_second_precision_utc() {
        let dt = datetime!(2025-01-02 03:04:05.678 UTC);
        assert_eq!(format_sas_time(dt), "2025-01-02T03:04:05Z");
    }

    #[test]
    fn appends_simple_path() {
        assert_eq!(
            append_path(
                "https://acct.blob.core.windows.net",
                "my-container/my blob.txt"
            ),
            "https://acct.blob.core.windows.net/my-container/my%20blob.txt"
        );
    }

    #[test]
    fn appends_path_handles_trailing_slash() {
        assert_eq!(
            append_path("https://acct.blob.core.windows.net/", "container"),
            "https://acct.blob.core.windows.net/container"
        );
    }

    #[test]
    fn appends_empty_path_returns_endpoint() {
        assert_eq!(
            append_path("https://acct.blob.core.windows.net", ""),
            "https://acct.blob.core.windows.net"
        );
    }

    #[test]
    fn encodes_reserved_characters() {
        // '?' '#' '+' '&' must be encoded.
        assert_eq!(encode_segment("a?b#c+d&e"), "a%3Fb%23c%2Bd%26e");
    }
}
