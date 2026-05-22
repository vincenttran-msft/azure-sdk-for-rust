// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::builder::Fields;
use crate::resource::{sealed, DelegatedResource, Resource};
use crate::UserDelegationKey;
use crate::SAS_VERSION;
use std::fmt;

/// A queue resource for user delegation SAS.
pub struct Queue {
    queue: String,
}

impl Queue {
    /// Creates a new queue resource.
    pub fn new(queue: impl Into<String>) -> Self {
        Self {
            queue: queue.into(),
        }
    }

    fn canonicalized_resource(&self, account: &str) -> String {
        format!("/queue/{}/{}", account, self.queue)
    }
}

/// Permissions for a queue SAS.
///
/// Serialization order: `raup`.
#[derive(Clone, Copy, Default)]
pub struct QueuePermissions {
    pub read: bool,
    pub add: bool,
    pub update: bool,
    pub process: bool,
}

impl fmt::Display for QueuePermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.read {
            f.write_str("r")?;
        }
        if self.add {
            f.write_str("a")?;
        }
        if self.update {
            f.write_str("u")?;
        }
        if self.process {
            f.write_str("p")?;
        }
        Ok(())
    }
}

impl sealed::Sealed for Queue {}
impl DelegatedResource for Queue {}

impl Resource for Queue {
    type Permissions = QueuePermissions;
    type SigningContext = UserDelegationKey;

    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
    ) -> String {
        // Queue UDK string-to-sign (15 fields):
        // sp, st, se, canonicalizedResource, skoid, sktid, skt, ske, sks, skv,
        // skdutid, sduoid, sip, spr, sv
        format!(
            "{sp}\n{st}\n{se}\n{cr}\n\
             {skoid}\n{sktid}\n{skt}\n{ske}\n{sks}\n{skv}\n\
             {skdutid}\n{sduoid}\n\
             {sip}\n{spr}\n{sv}\n",
            sp = permissions,
            st = fields.start_str(),
            se = fields.expiry_str(),
            cr = self.canonicalized_resource(&fields.account),
            skoid = context.signed_oid,
            sktid = context.signed_tid,
            skt = Fields::format_time(&context.signed_start),
            ske = Fields::format_time(&context.signed_expiry),
            sks = context.signed_service,
            skv = context.signed_version,
            skdutid = fields.delegated_tenant_id.as_deref().unwrap_or(""),
            sduoid = fields.delegated_user_object_id.as_deref().unwrap_or(""),
            sip = fields.ip_str(),
            spr = fields.protocol_str(),
            sv = SAS_VERSION,
        )
    }

    fn _build_query_parameters(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
        signature: &str,
    ) -> String {
        // Order: sv, st, se, sp, sip, spr, skoid, sktid, skt, ske, sks, skv, skdutid, sduoid, sig
        let mut parts = Vec::with_capacity(15);
        parts.push(format!("sv={SAS_VERSION}"));
        if let Some(ref start) = fields.start {
            parts.push(format!("st={}", Fields::format_time(start)));
        }
        parts.push(format!("se={}", fields.expiry_str()));
        parts.push(format!("sp={permissions}"));
        if let Some(ref ip) = fields.ip_range {
            parts.push(format!("sip={ip}"));
        }
        if let Some(ref proto) = fields.protocol {
            parts.push(format!("spr={proto}"));
        }
        parts.push(format!("skoid={}", context.signed_oid));
        parts.push(format!("sktid={}", context.signed_tid));
        parts.push(format!(
            "skt={}",
            Fields::format_time(&context.signed_start)
        ));
        parts.push(format!(
            "ske={}",
            Fields::format_time(&context.signed_expiry)
        ));
        parts.push(format!("sks={}", context.signed_service));
        parts.push(format!("skv={}", context.signed_version));
        if let Some(ref v) = fields.delegated_tenant_id {
            parts.push(format!("skdutid={v}"));
        }
        if let Some(ref v) = fields.delegated_user_object_id {
            parts.push(format!("sduoid={v}"));
        }
        parts.push(format!("sig={}", Fields::encode(signature)));
        parts.join("&")
    }
}
