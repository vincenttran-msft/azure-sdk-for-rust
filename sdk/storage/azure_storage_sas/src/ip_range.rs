// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::{fmt, net::IpAddr};

/// The range of allowed client IP addresses for a SAS token.
///
/// May be a single address or an inclusive range. Emitted as the `sip` query
/// parameter.
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
}
