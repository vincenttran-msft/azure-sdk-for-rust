// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::{fmt, net::IpAddr, str::FromStr};

use azure_core::{error::ErrorKind, Error, Result};

/// The range of allowed client IP addresses for a SAS token.
///
/// May be a single address or an inclusive range. Emitted as the `sip` query
/// parameter.
///
/// Parse from the canonical `"addr"` or `"addr-addr"` form with [`str::parse`]:
///
/// ```
/// use azure_storage_sas::SasIpRange;
/// let r: SasIpRange = "10.0.0.1-10.0.0.255".parse()?;
/// assert_eq!(r.to_string(), "10.0.0.1-10.0.0.255");
/// # Ok::<_, azure_core::Error>(())
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SasIpRange {
    start: IpAddr,
    end: Option<IpAddr>,
}

impl SasIpRange {
    /// Restrict the SAS to a single client IP address.
    pub fn single(address: IpAddr) -> Self {
        Self {
            start: address,
            end: None,
        }
    }

    /// Restrict the SAS to an inclusive range of client IP addresses.
    pub fn range(start: IpAddr, end: IpAddr) -> Self {
        Self {
            start,
            end: Some(end),
        }
    }
}

impl fmt::Display for SasIpRange {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.end {
            Some(end) => write!(f, "{}-{}", self.start, end),
            None => write!(f, "{}", self.start),
        }
    }
}

impl FromStr for SasIpRange {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let parse_addr = |part: &str| {
            part.parse::<IpAddr>().map_err(|e| {
                Error::with_message(
                    ErrorKind::DataConversion,
                    format!("invalid IP address '{part}': {e}"),
                )
            })
        };
        match s.split_once('-') {
            None => Ok(Self::single(parse_addr(s)?)),
            Some((start, end)) => {
                let start = parse_addr(start)?;
                let end = parse_addr(end)?;
                Ok(Self::range(start, end))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_single() {
        let r = SasIpRange::single("10.0.0.1".parse().unwrap());
        assert_eq!(r.to_string(), "10.0.0.1");
    }

    #[test]
    fn formats_range() {
        let r = SasIpRange::range("10.0.0.1".parse().unwrap(), "10.0.0.255".parse().unwrap());
        assert_eq!(r.to_string(), "10.0.0.1-10.0.0.255");
    }

    #[test]
    fn parses_single() {
        let r: SasIpRange = "10.0.0.1".parse().unwrap();
        assert_eq!(r, SasIpRange::single("10.0.0.1".parse().unwrap()));
    }

    #[test]
    fn parses_range() {
        let r: SasIpRange = "10.0.0.1-10.0.0.255".parse().unwrap();
        assert_eq!(
            r,
            SasIpRange::range("10.0.0.1".parse().unwrap(), "10.0.0.255".parse().unwrap())
        );
    }

    #[test]
    fn parses_ipv6_single() {
        let r: SasIpRange = "::1".parse().unwrap();
        assert_eq!(r.to_string(), "::1");
    }

    #[test]
    fn rejects_invalid_address() {
        let err = "not-an-ip".parse::<SasIpRange>().unwrap_err();
        assert!(err.to_string().contains("invalid IP address"));
    }

    #[test]
    fn round_trips() {
        for s in ["10.0.0.1", "10.0.0.1-10.0.0.255", "::1", "::1-::ffff"] {
            let r: SasIpRange = s.parse().unwrap();
            assert_eq!(r.to_string(), s);
        }
    }
}
