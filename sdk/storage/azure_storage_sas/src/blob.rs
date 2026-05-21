// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::fmt;

use azure_core::Result;
use percent_encoding::{utf8_percent_encode, AsciiSet, CONTROLS};
use time::OffsetDateTime;

use crate::{
    common::{append_path, format_sas_time, sign},
    ip_range::SasIpRange,
    key::UserDelegationKey,
    protocol::SasProtocol,
    NoKey, WithKey, SAS_VERSION,
};

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

fn encode_query(value: &str) -> String {
    utf8_percent_encode(value, QUERY).to_string()
}

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// Permissions that can be granted by a SAS for a single blob.
///
/// Letters are emitted in the canonical Azure order: `racwdxytmeopi`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlobSasPermissions {
    /// Read content, properties, metadata and block list.
    pub read: bool,
    /// Add a block to an append blob.
    pub add: bool,
    /// Write a new blob, snapshot, or copy.
    pub create: bool,
    /// Create or write content, properties, metadata, or block list.
    pub write: bool,
    /// Delete the blob.
    pub delete: bool,
    /// Delete a blob version.
    pub delete_version: bool,
    /// Permanently delete a soft-deleted blob.
    pub permanent_delete: bool,
    /// Read or write blob tags.
    pub tag: bool,
    /// Move (rename) a blob in a hierarchical-namespace account.
    pub move_blob: bool,
    /// Execute a blob (HNS only).
    pub execute: bool,
    /// Manage ownership (HNS only).
    pub ownership: bool,
    /// Manage permissions/ACLs (HNS only).
    pub permissions: bool,
    /// Set or extend an immutability policy on the blob.
    pub set_immutability_policy: bool,
}

impl BlobSasPermissions {
    pub(crate) fn to_permission_string(self) -> String {
        let mut s = String::with_capacity(13);
        if self.read {
            s.push('r');
        }
        if self.add {
            s.push('a');
        }
        if self.create {
            s.push('c');
        }
        if self.write {
            s.push('w');
        }
        if self.delete {
            s.push('d');
        }
        if self.delete_version {
            s.push('x');
        }
        if self.permanent_delete {
            s.push('y');
        }
        if self.tag {
            s.push('t');
        }
        if self.move_blob {
            s.push('m');
        }
        if self.execute {
            s.push('e');
        }
        if self.ownership {
            s.push('o');
        }
        if self.permissions {
            s.push('p');
        }
        if self.set_immutability_policy {
            s.push('i');
        }
        s
    }
}

/// Permissions that can be granted by a SAS for a blob container.
///
/// Letters are emitted in the canonical Azure order: `racwdxyltmeopi`.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct BlobContainerSasPermissions {
    /// Read content, properties, metadata, and block list of any blob.
    pub read: bool,
    /// Add a block to an append blob.
    pub add: bool,
    /// Write a new blob, snapshot, or copy.
    pub create: bool,
    /// Create or write content, properties, metadata, or block list.
    pub write: bool,
    /// Delete any blob in the container.
    pub delete: bool,
    /// Delete a blob version.
    pub delete_version: bool,
    /// Permanently delete a soft-deleted blob.
    pub permanent_delete: bool,
    /// List blobs in the container.
    pub list: bool,
    /// Read or write blob tags.
    pub tag: bool,
    /// Move (rename) a blob in a hierarchical-namespace account.
    pub move_blob: bool,
    /// Execute a blob (HNS only).
    pub execute: bool,
    /// Manage ownership (HNS only).
    pub ownership: bool,
    /// Manage permissions/ACLs (HNS only).
    pub permissions: bool,
    /// Set or extend an immutability policy on a blob.
    pub set_immutability_policy: bool,
}

impl BlobContainerSasPermissions {
    pub(crate) fn to_permission_string(self) -> String {
        let mut s = String::with_capacity(14);
        if self.read {
            s.push('r');
        }
        if self.add {
            s.push('a');
        }
        if self.create {
            s.push('c');
        }
        if self.write {
            s.push('w');
        }
        if self.delete {
            s.push('d');
        }
        if self.delete_version {
            s.push('x');
        }
        if self.permanent_delete {
            s.push('y');
        }
        if self.list {
            s.push('l');
        }
        if self.tag {
            s.push('t');
        }
        if self.move_blob {
            s.push('m');
        }
        if self.execute {
            s.push('e');
        }
        if self.ownership {
            s.push('o');
        }
        if self.permissions {
            s.push('p');
        }
        if self.set_immutability_policy {
            s.push('i');
        }
        s
    }
}

// ---------------------------------------------------------------------------
// Builder common state
// ---------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
struct CommonValues {
    start: Option<OffsetDateTime>,
    protocol: Option<SasProtocol>,
    ip_range: Option<SasIpRange>,
    encryption_scope: Option<String>,
    correlation_id: Option<String>,
    preauthorized_agent_object_id: Option<String>,
    agent_object_id: Option<String>,
    cache_control: Option<String>,
    content_disposition: Option<String>,
    content_encoding: Option<String>,
    content_language: Option<String>,
    content_type: Option<String>,
}

// ---------------------------------------------------------------------------
// BlobSasBuilder
// ---------------------------------------------------------------------------

/// Builder for a user delegation SAS targeting a single blob.
///
/// Construct with [`BlobSasBuilder::new`], chain optional setters, attach a
/// [`UserDelegationKey`] with [`with_key`](BlobSasBuilder::with_key), then call
/// [`sign`](BlobSasBuilder::sign) to produce a signed
/// [`BlobSasQueryParameters`].
///
/// The generic `S` parameter is a type-state marker: a freshly-constructed
/// builder is `BlobSasBuilder<NoKey>` and exposes no signing methods. Calling
/// `.with_key(...)` transitions to `BlobSasBuilder<WithKey>`, at which point
/// `.sign(...)` becomes available.
#[derive(Debug)]
pub struct BlobSasBuilder<S = NoKey> {
    container_name: String,
    blob_name: String,
    expiry: OffsetDateTime,
    permissions: BlobSasPermissions,
    common: CommonValues,
    state: S,
}

impl BlobSasBuilder<NoKey> {
    /// Create a new builder for the given blob with the required fields.
    ///
    /// `blob_name` must be the unencoded blob name; backslashes are normalized
    /// to forward slashes.
    pub fn new(
        container_name: String,
        blob_name: String,
        expiry: OffsetDateTime,
        permissions: BlobSasPermissions,
    ) -> Self {
        Self {
            container_name,
            blob_name: blob_name.replace('\\', "/"),
            expiry,
            permissions,
            common: CommonValues::default(),
            state: NoKey,
        }
    }

    /// Attach a user delegation key, transitioning the builder so that
    /// [`sign`](BlobSasBuilder::sign) is callable.
    pub fn with_key(self, key: UserDelegationKey) -> BlobSasBuilder<WithKey> {
        BlobSasBuilder {
            container_name: self.container_name,
            blob_name: self.blob_name,
            expiry: self.expiry,
            permissions: self.permissions,
            common: self.common,
            state: WithKey(key),
        }
    }
}

impl<S> BlobSasBuilder<S> {
    common_setters!();
}

impl BlobSasBuilder<WithKey> {
    /// Compute the signature and return the resulting
    /// [`BlobSasQueryParameters`].
    ///
    /// `account_name` is the storage account name (no domain). It is included
    /// in the canonical resource string but is *not* itself signed as a
    /// separate field.
    pub fn sign(self, account_name: &str) -> Result<BlobSasQueryParameters> {
        let key = &self.state.0;
        let permissions = self.permissions.to_permission_string();
        let canonical_resource = format!(
            "/blob/{}/{}/{}",
            account_name, self.container_name, self.blob_name
        );
        let string_to_sign = build_string_to_sign(
            &permissions,
            self.common.start,
            self.expiry,
            &canonical_resource,
            key,
            &self.common,
            "b",
            "", // signed_snapshot_time
        );

        let signature = sign(&string_to_sign, &key.value)?;

        Ok(BlobSasQueryParameters {
            permissions,
            start: self.common.start,
            expiry: self.expiry,
            ip_range: self.common.ip_range,
            protocol: self.common.protocol,
            resource: "b",
            key: key.clone(),
            correlation_id: self.common.correlation_id,
            preauthorized_agent_object_id: self.common.preauthorized_agent_object_id,
            agent_object_id: self.common.agent_object_id,
            encryption_scope: self.common.encryption_scope,
            cache_control: self.common.cache_control,
            content_disposition: self.common.content_disposition,
            content_encoding: self.common.content_encoding,
            content_language: self.common.content_language,
            content_type: self.common.content_type,
            signature,
            container_name: self.container_name,
            blob_name: Some(self.blob_name),
        })
    }
}

// ---------------------------------------------------------------------------
// BlobContainerSasBuilder
// ---------------------------------------------------------------------------

/// Builder for a user delegation SAS targeting a blob container.
#[derive(Debug)]
pub struct BlobContainerSasBuilder<S = NoKey> {
    container_name: String,
    expiry: OffsetDateTime,
    permissions: BlobContainerSasPermissions,
    common: CommonValues,
    state: S,
}

impl BlobContainerSasBuilder<NoKey> {
    /// Create a new builder for the given container with the required fields.
    pub fn new(
        container_name: String,
        expiry: OffsetDateTime,
        permissions: BlobContainerSasPermissions,
    ) -> Self {
        Self {
            container_name,
            expiry,
            permissions,
            common: CommonValues::default(),
            state: NoKey,
        }
    }

    /// Attach a user delegation key, transitioning the builder so that
    /// [`sign`](BlobContainerSasBuilder::sign) is callable.
    pub fn with_key(self, key: UserDelegationKey) -> BlobContainerSasBuilder<WithKey> {
        BlobContainerSasBuilder {
            container_name: self.container_name,
            expiry: self.expiry,
            permissions: self.permissions,
            common: self.common,
            state: WithKey(key),
        }
    }
}

impl<S> BlobContainerSasBuilder<S> {
    common_setters!();
}

impl BlobContainerSasBuilder<WithKey> {
    /// Compute the signature and return the resulting
    /// [`BlobSasQueryParameters`].
    pub fn sign(self, account_name: &str) -> Result<BlobSasQueryParameters> {
        let key = &self.state.0;
        let permissions = self.permissions.to_permission_string();
        let canonical_resource = format!("/blob/{}/{}", account_name, self.container_name);
        let string_to_sign = build_string_to_sign(
            &permissions,
            self.common.start,
            self.expiry,
            &canonical_resource,
            key,
            &self.common,
            "c",
            "",
        );

        let signature = sign(&string_to_sign, &key.value)?;

        Ok(BlobSasQueryParameters {
            permissions,
            start: self.common.start,
            expiry: self.expiry,
            ip_range: self.common.ip_range,
            protocol: self.common.protocol,
            resource: "c",
            key: key.clone(),
            correlation_id: self.common.correlation_id,
            preauthorized_agent_object_id: self.common.preauthorized_agent_object_id,
            agent_object_id: self.common.agent_object_id,
            encryption_scope: self.common.encryption_scope,
            cache_control: self.common.cache_control,
            content_disposition: self.common.content_disposition,
            content_encoding: self.common.content_encoding,
            content_language: self.common.content_language,
            content_type: self.common.content_type,
            signature,
            container_name: self.container_name,
            blob_name: None,
        })
    }
}

// ---------------------------------------------------------------------------
// Shared setter macro
// ---------------------------------------------------------------------------

macro_rules! common_setters {
    () => {
        /// Set the start time of the SAS.
        pub fn start(mut self, value: OffsetDateTime) -> Self {
            self.common.start = Some(value);
            self
        }
        /// Set the allowed protocol(s).
        pub fn protocol(mut self, value: SasProtocol) -> Self {
            self.common.protocol = Some(value);
            self
        }
        /// Restrict the SAS to a single IP or range.
        pub fn ip_range(mut self, value: SasIpRange) -> Self {
            self.common.ip_range = Some(value);
            self
        }
        /// Set the encryption scope (emitted as `ses`).
        pub fn encryption_scope(mut self, value: impl Into<String>) -> Self {
            self.common.encryption_scope = Some(value.into());
            self
        }
        /// Set the correlation id (emitted as `scid`).
        pub fn correlation_id(mut self, value: impl Into<String>) -> Self {
            self.common.correlation_id = Some(value.into());
            self
        }
        /// Set the preauthorized agent AAD object id (emitted as `saoid`).
        pub fn preauthorized_agent_object_id(mut self, value: impl Into<String>) -> Self {
            self.common.preauthorized_agent_object_id = Some(value.into());
            self
        }
        /// Set the agent AAD object id (emitted as `suoid`).
        pub fn agent_object_id(mut self, value: impl Into<String>) -> Self {
            self.common.agent_object_id = Some(value.into());
            self
        }
        /// Override the response `Cache-Control` header (emitted as `rscc`).
        pub fn cache_control(mut self, value: impl Into<String>) -> Self {
            self.common.cache_control = Some(value.into());
            self
        }
        /// Override the response `Content-Disposition` header (emitted as `rscd`).
        pub fn content_disposition(mut self, value: impl Into<String>) -> Self {
            self.common.content_disposition = Some(value.into());
            self
        }
        /// Override the response `Content-Encoding` header (emitted as `rsce`).
        pub fn content_encoding(mut self, value: impl Into<String>) -> Self {
            self.common.content_encoding = Some(value.into());
            self
        }
        /// Override the response `Content-Language` header (emitted as `rscl`).
        pub fn content_language(mut self, value: impl Into<String>) -> Self {
            self.common.content_language = Some(value.into());
            self
        }
        /// Override the response `Content-Type` header (emitted as `rsct`).
        pub fn content_type(mut self, value: impl Into<String>) -> Self {
            self.common.content_type = Some(value.into());
            self
        }
    };
}
use common_setters;

// ---------------------------------------------------------------------------
// String to sign
// ---------------------------------------------------------------------------

#[allow(clippy::too_many_arguments)]
fn build_string_to_sign(
    permissions: &str,
    start: Option<OffsetDateTime>,
    expiry: OffsetDateTime,
    canonical_resource: &str,
    key: &UserDelegationKey,
    common: &CommonValues,
    resource: &str,
    signed_snapshot_time: &str,
) -> String {
    let start_str = start.map(format_sas_time).unwrap_or_default();
    let expiry_str = format_sas_time(expiry);
    let key_start = format_sas_time(key.signed_start);
    let key_expiry = format_sas_time(key.signed_expiry);
    let ip = common
        .ip_range
        .as_ref()
        .map(|r| r.to_string())
        .unwrap_or_default();
    let protocol = common
        .protocol
        .map(|p| p.as_str().to_string())
        .unwrap_or_default();

    let fields = [
        permissions,
        &start_str,
        &expiry_str,
        canonical_resource,
        &key.signed_object_id,
        &key.signed_tenant_id,
        &key_start,
        &key_expiry,
        &key.signed_service,
        &key.signed_version,
        common
            .preauthorized_agent_object_id
            .as_deref()
            .unwrap_or(""),
        common.agent_object_id.as_deref().unwrap_or(""),
        common.correlation_id.as_deref().unwrap_or(""),
        "", // signed_key_delegated_user_tenant_id (skdtid) — not yet supported
        "", // signed_delegated_user_object_id (sduoid) — not yet supported
        &ip,
        &protocol,
        SAS_VERSION,
        resource,
        signed_snapshot_time,
        common.encryption_scope.as_deref().unwrap_or(""),
        common.cache_control.as_deref().unwrap_or(""),
        common.content_disposition.as_deref().unwrap_or(""),
        common.content_encoding.as_deref().unwrap_or(""),
        common.content_language.as_deref().unwrap_or(""),
        common.content_type.as_deref().unwrap_or(""),
    ];
    fields.join("\n")
}

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

/// A signed SAS token for blob storage.
///
/// Render it as a URL query string via [`fmt::Display`], or attach it to a
/// blob/container endpoint with [`to_url`](BlobSasQueryParameters::to_url).
#[derive(Clone, Debug)]
pub struct BlobSasQueryParameters {
    permissions: String,
    start: Option<OffsetDateTime>,
    expiry: OffsetDateTime,
    ip_range: Option<SasIpRange>,
    protocol: Option<SasProtocol>,
    resource: &'static str,
    key: UserDelegationKey,
    correlation_id: Option<String>,
    preauthorized_agent_object_id: Option<String>,
    agent_object_id: Option<String>,
    encryption_scope: Option<String>,
    cache_control: Option<String>,
    content_disposition: Option<String>,
    content_encoding: Option<String>,
    content_language: Option<String>,
    content_type: Option<String>,
    signature: String,
    container_name: String,
    blob_name: Option<String>,
}

impl BlobSasQueryParameters {
    /// The signed permissions string (the `sp` value).
    pub fn permissions(&self) -> &str {
        &self.permissions
    }

    /// The container that the SAS targets.
    pub fn container_name(&self) -> &str {
        &self.container_name
    }

    /// The blob that the SAS targets, if this is a blob-scoped SAS.
    pub fn blob_name(&self) -> Option<&str> {
        self.blob_name.as_deref()
    }

    /// The computed signature (the `sig` value, base64 encoded).
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Combine the SAS query string with the given storage endpoint
    /// (e.g. `https://myaccount.blob.core.windows.net`) and the target
    /// container/blob path to produce a complete URL.
    pub fn to_url(&self, endpoint: &str) -> Result<String> {
        let mut path = self.container_name.clone();
        if let Some(blob) = &self.blob_name {
            path.push('/');
            path.push_str(blob);
        }
        let base = append_path(endpoint, &path);
        Ok(format!("{}?{}", base, self))
    }
}

impl fmt::Display for BlobSasQueryParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        let mut write_pair = |f: &mut fmt::Formatter<'_>, k: &str, v: &str| -> fmt::Result {
            if v.is_empty() {
                return Ok(());
            }
            if !first {
                f.write_str("&")?;
            }
            first = false;
            write!(f, "{}={}", k, encode_query(v))
        };

        write_pair(f, "sv", SAS_VERSION)?;
        write_pair(f, "sr", self.resource)?;
        if let Some(start) = self.start {
            write_pair(f, "st", &format_sas_time(start))?;
        }
        write_pair(f, "se", &format_sas_time(self.expiry))?;
        write_pair(f, "sp", &self.permissions)?;
        if let Some(ip) = &self.ip_range {
            write_pair(f, "sip", &ip.to_string())?;
        }
        if let Some(p) = self.protocol {
            write_pair(f, "spr", p.as_str())?;
        }
        write_pair(f, "skoid", &self.key.signed_object_id)?;
        write_pair(f, "sktid", &self.key.signed_tenant_id)?;
        write_pair(f, "skt", &format_sas_time(self.key.signed_start))?;
        write_pair(f, "ske", &format_sas_time(self.key.signed_expiry))?;
        write_pair(f, "sks", &self.key.signed_service)?;
        write_pair(f, "skv", &self.key.signed_version)?;
        if let Some(v) = &self.preauthorized_agent_object_id {
            write_pair(f, "saoid", v)?;
        }
        if let Some(v) = &self.agent_object_id {
            write_pair(f, "suoid", v)?;
        }
        if let Some(v) = &self.correlation_id {
            write_pair(f, "scid", v)?;
        }
        if let Some(v) = &self.encryption_scope {
            write_pair(f, "ses", v)?;
        }
        if let Some(v) = &self.cache_control {
            write_pair(f, "rscc", v)?;
        }
        if let Some(v) = &self.content_disposition {
            write_pair(f, "rscd", v)?;
        }
        if let Some(v) = &self.content_encoding {
            write_pair(f, "rsce", v)?;
        }
        if let Some(v) = &self.content_language {
            write_pair(f, "rscl", v)?;
        }
        if let Some(v) = &self.content_type {
            write_pair(f, "rsct", v)?;
        }
        write_pair(f, "sig", &self.signature)?;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use time::macros::datetime;

    fn sample_key() -> UserDelegationKey {
        UserDelegationKey::new(
            "00000000-0000-0000-0000-000000000001",
            "00000000-0000-0000-0000-000000000002",
            datetime!(2025-01-01 00:00:00 UTC),
            datetime!(2025-01-08 00:00:00 UTC),
            "b",
            SAS_VERSION,
            // 32 zero bytes, base64 — deterministic test key.
            "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA=",
        )
    }

    #[test]
    fn permissions_emit_in_canonical_order() {
        let p = BlobSasPermissions {
            write: true,
            read: true,
            delete: true,
            ..Default::default()
        };
        assert_eq!(p.to_permission_string(), "rwd");
    }

    #[test]
    fn container_permissions_include_list() {
        let p = BlobContainerSasPermissions {
            read: true,
            list: true,
            ..Default::default()
        };
        assert_eq!(p.to_permission_string(), "rl");
    }

    #[test]
    fn blob_sas_signs_and_renders_query() {
        let mut perms = BlobSasPermissions::default();
        perms.read = true;

        let sas = BlobSasBuilder::new(
            "my-container".into(),
            "my blob.txt".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .start(datetime!(2025-01-01 00:00:00 UTC))
        .protocol(SasProtocol::Https)
        .with_key(sample_key())
        .sign("myacct")
        .expect("sign");

        let query = sas.to_string();
        assert!(query.starts_with(&format!("sv={}", SAS_VERSION)));
        assert!(query.contains("&sr=b"));
        assert!(query.contains("&sp=r"));
        assert!(query.contains("&spr=https"));
        assert!(query.contains("&skoid=00000000-0000-0000-0000-000000000001"));
        // Signature must be present and base64 (length 44 for SHA-256).
        let sig = sas.signature();
        assert_eq!(sig.len(), 44);
    }

    #[test]
    fn blob_sas_signature_is_deterministic() {
        let mut perms = BlobSasPermissions::default();
        perms.read = true;
        let build = || {
            BlobSasBuilder::new(
                "c".into(),
                "b".into(),
                datetime!(2025-01-02 00:00:00 UTC),
                perms,
            )
            .start(datetime!(2025-01-01 00:00:00 UTC))
            .with_key(sample_key())
            .sign("acct")
            .unwrap()
        };
        assert_eq!(build().signature(), build().signature());
    }

    #[test]
    fn container_sas_to_url() {
        let mut perms = BlobContainerSasPermissions::default();
        perms.read = true;
        perms.list = true;
        let sas = BlobContainerSasBuilder::new(
            "my-container".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap();

        let url = sas.to_url("https://acct.blob.core.windows.net").unwrap();
        assert!(url.starts_with("https://acct.blob.core.windows.net/my-container?sv="));
        assert!(url.contains("&sr=c"));
    }

    #[test]
    fn blob_name_with_backslashes_is_normalized() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let sas = BlobSasBuilder::new(
            "c".into(),
            r"a\b\c.txt".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        assert_eq!(sas.blob_name(), Some("a/b/c.txt"));
    }
}
