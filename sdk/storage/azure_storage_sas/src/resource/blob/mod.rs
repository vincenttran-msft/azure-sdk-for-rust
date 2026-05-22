// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Blob-service resource types: blob, container, and directory.

mod blob_resource;
mod container;
mod directory;

pub use blob_resource::{Blob, BlobPermissions};
pub use container::Container;
pub use directory::Directory;

/// Permissions shared by container and directory resources.
///
/// Serialization order: `racwdxyltmeopi`.
pub use container::ContainerPermissions;

use crate::builder::Fields;
use crate::UserDelegationKey;
use crate::SAS_VERSION;

/// Builds the blob-service user delegation SAS string-to-sign (26 fields).
///
/// Used by all blob-service resource types (blob, snapshot, version, container, directory).
pub(crate) fn blob_udk_string_to_sign(
    permissions: &impl std::fmt::Display,
    fields: &Fields,
    key: &UserDelegationKey,
    sr: &str,
    canonicalized_resource: &str,
    snapshot_time: &str,
    directory_depth: &str,
) -> String {
    // Fields: sp, st, se, canonicalizedResource, skoid, sktid, skt, ske, sks, skv,
    //         saoid, suoid, scid, skdutid, sduoid, sip, spr, sv, sr, snapshotTime,
    //         ses, rscc, rscd, rsce, rscl, rsct
    let _ = directory_depth; // Used only in query parameters (sdd), not in string-to-sign.
    format!(
        "{sp}\n{st}\n{se}\n{cr}\n{skoid}\n{sktid}\n{skt}\n{ske}\n{sks}\n{skv}\n\
         {saoid}\n{suoid}\n{scid}\n{skdutid}\n{sduoid}\n\
         {sip}\n{spr}\n{sv}\n{sr}\n{snapshot}\n\
         {ses}\n{rscc}\n{rscd}\n{rsce}\n{rscl}\n{rsct}\n",
        sp = permissions,
        st = fields.start_str(),
        se = fields.expiry_str(),
        cr = canonicalized_resource,
        skoid = key.signed_oid,
        sktid = key.signed_tid,
        skt = Fields::format_time(&key.signed_start),
        ske = Fields::format_time(&key.signed_expiry),
        sks = key.signed_service,
        skv = key.signed_version,
        saoid = fields.authorized_object_id.as_deref().unwrap_or(""),
        suoid = fields.unauthorized_object_id.as_deref().unwrap_or(""),
        scid = fields.correlation_id.as_deref().unwrap_or(""),
        skdutid = fields.delegated_tenant_id.as_deref().unwrap_or(""),
        sduoid = fields.delegated_user_object_id.as_deref().unwrap_or(""),
        sip = fields.ip_str(),
        spr = fields.protocol_str(),
        sv = SAS_VERSION,
        snapshot = snapshot_time,
        ses = fields.encryption_scope_str(),
        rscc = fields.cache_control.as_deref().unwrap_or(""),
        rscd = fields.content_disposition.as_deref().unwrap_or(""),
        rsce = fields.content_encoding.as_deref().unwrap_or(""),
        rscl = fields.content_language.as_deref().unwrap_or(""),
        rsct = fields.content_type.as_deref().unwrap_or(""),
    )
}

/// Builds the blob-service user delegation SAS query parameters.
pub(crate) fn blob_udk_query_parameters(
    permissions: &impl std::fmt::Display,
    fields: &Fields,
    key: &UserDelegationKey,
    sr: &str,
    snapshot_time: Option<&str>,
    directory_depth: Option<u32>,
    signature: &str,
) -> String {
    // Order: sv, sr, st, se, sp, sip, spr, skoid, sktid, skt, ske, sks, skv,
    //        saoid, suoid, scid, skdutid, sduoid, ses, sdd, rscc, rscd, rsce, rscl, rsct, sig
    let mut parts = Vec::with_capacity(26);
    parts.push(format!("sv={SAS_VERSION}"));
    parts.push(format!("sr={sr}"));
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
    parts.push(format!("skoid={}", key.signed_oid));
    parts.push(format!("sktid={}", key.signed_tid));
    parts.push(format!("skt={}", Fields::format_time(&key.signed_start)));
    parts.push(format!("ske={}", Fields::format_time(&key.signed_expiry)));
    parts.push(format!("sks={}", key.signed_service));
    parts.push(format!("skv={}", key.signed_version));
    if let Some(ref v) = fields.authorized_object_id {
        parts.push(format!("saoid={v}"));
    }
    if let Some(ref v) = fields.unauthorized_object_id {
        parts.push(format!("suoid={v}"));
    }
    if let Some(ref v) = fields.correlation_id {
        parts.push(format!("scid={v}"));
    }
    if let Some(ref v) = fields.delegated_tenant_id {
        parts.push(format!("skdutid={v}"));
    }
    if let Some(ref v) = fields.delegated_user_object_id {
        parts.push(format!("sduoid={v}"));
    }
    if let Some(ref v) = fields.encryption_scope {
        parts.push(format!("ses={v}"));
    }
    if let Some(depth) = directory_depth {
        parts.push(format!("sdd={depth}"));
    }
    if let Some(v) = snapshot_time {
        parts.push(format!("snapshot={v}"));
    }
    if let Some(ref v) = fields.cache_control {
        parts.push(format!("rscc={v}"));
    }
    if let Some(ref v) = fields.content_disposition {
        parts.push(format!("rscd={v}"));
    }
    if let Some(ref v) = fields.content_encoding {
        parts.push(format!("rsce={v}"));
    }
    if let Some(ref v) = fields.content_language {
        parts.push(format!("rscl={v}"));
    }
    if let Some(ref v) = fields.content_type {
        parts.push(format!("rsct={v}"));
    }
    parts.push(format!("sig={}", Fields::encode(signature)));
    parts.join("&")
}
