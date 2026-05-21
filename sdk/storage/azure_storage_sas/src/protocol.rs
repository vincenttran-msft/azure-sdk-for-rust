// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::fmt;

/// The protocol(s) over which the SAS token will be honored.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum SasProtocol {
    /// Allow HTTPS only. This is the default.
    #[default]
    Https,
    /// Allow both HTTPS and HTTP.
    HttpsAndHttp,
}

impl SasProtocol {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            SasProtocol::Https => "https",
            SasProtocol::HttpsAndHttp => "https,http",
        }
    }
}

impl fmt::Display for SasProtocol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
