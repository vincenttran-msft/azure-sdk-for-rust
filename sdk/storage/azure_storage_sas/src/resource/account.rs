// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::builder::Fields;
use crate::resource::{sealed, Resource};
use crate::SAS_VERSION;
use std::fmt;

/// An account-level SAS resource.
///
/// Account SAS grants access to resources in one or more storage services.
pub struct Account {
    /// The services accessible with this SAS.
    pub services: AccountServices,
    /// The resource types accessible with this SAS.
    pub resource_types: AccountResourceTypes,
}

impl Account {
    /// Creates a new account SAS resource.
    pub fn new(services: AccountServices, resource_types: AccountResourceTypes) -> Self {
        Self {
            services,
            resource_types,
        }
    }
}

/// Storage services accessible via an account SAS.
#[derive(Clone, Copy, Default)]
pub struct AccountServices {
    /// Blob service (including ADLS Gen2).
    pub blob: bool,
    /// Queue service.
    pub queue: bool,
    /// Table service.
    pub table: bool,
    /// File service.
    pub file: bool,
}

impl fmt::Display for AccountServices {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.blob {
            f.write_str("b")?;
        }
        if self.queue {
            f.write_str("q")?;
        }
        if self.table {
            f.write_str("t")?;
        }
        if self.file {
            f.write_str("f")?;
        }
        Ok(())
    }
}

/// Resource types accessible via an account SAS.
#[derive(Clone, Copy, Default)]
pub struct AccountResourceTypes {
    /// Service-level APIs (e.g., list containers).
    pub service: bool,
    /// Container-level APIs (e.g., list blobs).
    pub container: bool,
    /// Object-level APIs (e.g., get blob).
    pub object: bool,
}

impl fmt::Display for AccountResourceTypes {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.service {
            f.write_str("s")?;
        }
        if self.container {
            f.write_str("c")?;
        }
        if self.object {
            f.write_str("o")?;
        }
        Ok(())
    }
}

/// Permissions for an account SAS.
///
/// The order of characters when serialized is: `rwdlacupf`.
#[derive(Clone, Copy, Default)]
pub struct AccountPermissions {
    pub read: bool,
    pub write: bool,
    pub delete: bool,
    pub list: bool,
    pub add: bool,
    pub create: bool,
    pub update: bool,
    pub process: bool,
    pub filter: bool,
}

impl fmt::Display for AccountPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.read {
            f.write_str("r")?;
        }
        if self.write {
            f.write_str("w")?;
        }
        if self.delete {
            f.write_str("d")?;
        }
        if self.list {
            f.write_str("l")?;
        }
        if self.add {
            f.write_str("a")?;
        }
        if self.create {
            f.write_str("c")?;
        }
        if self.update {
            f.write_str("u")?;
        }
        if self.process {
            f.write_str("p")?;
        }
        if self.filter {
            f.write_str("f")?;
        }
        Ok(())
    }
}

impl sealed::Sealed for Account {}

impl Resource for Account {
    type Permissions = AccountPermissions;
    type SigningContext = ();

    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        _context: &Self::SigningContext,
    ) -> String {
        // Account SAS string-to-sign (sv=2025-07-05+):
        // {account}\n{sp}\n{ss}\n{srt}\n{st}\n{se}\n{sip}\n{spr}\n{sv}\n{ses}\n
        format!(
            "{account}\n{sp}\n{ss}\n{srt}\n{st}\n{se}\n{sip}\n{spr}\n{sv}\n{ses}\n",
            account = fields.account,
            sp = permissions,
            ss = self.services,
            srt = self.resource_types,
            st = fields.start_str(),
            se = fields.expiry_str(),
            sip = fields.ip_str(),
            spr = fields.protocol_str(),
            sv = SAS_VERSION,
            ses = fields.encryption_scope_str(),
        )
    }

    fn _build_query_parameters(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        _context: &Self::SigningContext,
        signature: &str,
    ) -> String {
        // Order: sv, ss, srt, sp, st, se, sip, spr, ses, sig
        let mut parts = Vec::with_capacity(10);
        parts.push(format!("sv={SAS_VERSION}"));
        parts.push(format!("ss={}", self.services));
        parts.push(format!("srt={}", self.resource_types));
        parts.push(format!("sp={permissions}"));
        if let Some(ref start) = fields.start {
            parts.push(format!("st={}", Fields::format_time(start)));
        }
        parts.push(format!("se={}", fields.expiry_str()));
        if let Some(ref ip) = fields.ip_range {
            parts.push(format!("sip={ip}"));
        }
        if let Some(ref proto) = fields.protocol {
            parts.push(format!("spr={proto}"));
        }
        if let Some(ref ses) = fields.encryption_scope {
            parts.push(format!("ses={ses}"));
        }
        parts.push(format!("sig={}", Fields::encode(signature)));
        parts.join("&")
    }
}
