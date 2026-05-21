// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use std::{fmt, str::FromStr};

use azure_core::{error::ErrorKind, Error, Result};
use time::OffsetDateTime;

use crate::{
    common::{append_path, encode_query, format_sas_time, sign},
    ip_range::SasIpRange,
    key::UserDelegationKey,
    protocol::SasProtocol,
    NoKey, WithKey, SAS_VERSION,
};

// ---------------------------------------------------------------------------
// Permissions
// ---------------------------------------------------------------------------

/// Permissions that can be granted by a SAS for a single blob.
///
/// Letters are emitted in the canonical Azure order: `racwdxytmeopi`.
///
/// Parse a permission string with [`str::parse`]:
///
/// ```
/// use azure_storage_sas::BlobSasPermissions;
/// let p: BlobSasPermissions = "rwd".parse()?;
/// assert!(p.read && p.write && p.delete);
/// assert_eq!(p.to_string(), "rwd");
/// # Ok::<_, azure_core::Error>(())
/// ```
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
    /// Grant every blob permission.
    ///
    /// ```
    /// use azure_storage_sas::BlobSasPermissions;
    /// assert_eq!(BlobSasPermissions::all().to_string(), "racwdxytmeopi");
    /// ```
    pub fn all() -> Self {
        Self {
            read: true,
            add: true,
            create: true,
            write: true,
            delete: true,
            delete_version: true,
            permanent_delete: true,
            tag: true,
            move_blob: true,
            execute: true,
            ownership: true,
            permissions: true,
            set_immutability_policy: true,
        }
    }

    /// Grant only the `read` permission.
    ///
    /// ```
    /// use azure_storage_sas::BlobSasPermissions;
    /// assert_eq!(BlobSasPermissions::read_only().to_string(), "r");
    /// ```
    pub fn read_only() -> Self {
        Self {
            read: true,
            ..Default::default()
        }
    }

    pub(crate) fn to_permission_string(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for BlobSasPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.read {
            f.write_str("r")?;
        }
        if self.add {
            f.write_str("a")?;
        }
        if self.create {
            f.write_str("c")?;
        }
        if self.write {
            f.write_str("w")?;
        }
        if self.delete {
            f.write_str("d")?;
        }
        if self.delete_version {
            f.write_str("x")?;
        }
        if self.permanent_delete {
            f.write_str("y")?;
        }
        if self.tag {
            f.write_str("t")?;
        }
        if self.move_blob {
            f.write_str("m")?;
        }
        if self.execute {
            f.write_str("e")?;
        }
        if self.ownership {
            f.write_str("o")?;
        }
        if self.permissions {
            f.write_str("p")?;
        }
        if self.set_immutability_policy {
            f.write_str("i")?;
        }
        Ok(())
    }
}

impl FromStr for BlobSasPermissions {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut p = Self::default();
        for c in s.chars() {
            match c {
                'r' => p.read = true,
                'a' => p.add = true,
                'c' => p.create = true,
                'w' => p.write = true,
                'd' => p.delete = true,
                'x' => p.delete_version = true,
                'y' => p.permanent_delete = true,
                't' => p.tag = true,
                'm' => p.move_blob = true,
                'e' => p.execute = true,
                'o' => p.ownership = true,
                'p' => p.permissions = true,
                'i' => p.set_immutability_policy = true,
                other => {
                    return Err(Error::with_message(
                        ErrorKind::DataConversion,
                        format!("invalid blob SAS permission character '{other}'"),
                    ));
                }
            }
        }
        Ok(p)
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
    /// Grant every container permission.
    ///
    /// ```
    /// use azure_storage_sas::BlobContainerSasPermissions;
    /// assert_eq!(BlobContainerSasPermissions::all().to_string(), "racwdxyltmeopi");
    /// ```
    pub fn all() -> Self {
        Self {
            read: true,
            add: true,
            create: true,
            write: true,
            delete: true,
            delete_version: true,
            permanent_delete: true,
            list: true,
            tag: true,
            move_blob: true,
            execute: true,
            ownership: true,
            permissions: true,
            set_immutability_policy: true,
        }
    }

    /// Grant `read` and `list` permissions.
    ///
    /// ```
    /// use azure_storage_sas::BlobContainerSasPermissions;
    /// assert_eq!(BlobContainerSasPermissions::read_only().to_string(), "rl");
    /// ```
    pub fn read_only() -> Self {
        Self {
            read: true,
            list: true,
            ..Default::default()
        }
    }

    pub(crate) fn to_permission_string(self) -> String {
        self.to_string()
    }
}

impl fmt::Display for BlobContainerSasPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.read {
            f.write_str("r")?;
        }
        if self.add {
            f.write_str("a")?;
        }
        if self.create {
            f.write_str("c")?;
        }
        if self.write {
            f.write_str("w")?;
        }
        if self.delete {
            f.write_str("d")?;
        }
        if self.delete_version {
            f.write_str("x")?;
        }
        if self.permanent_delete {
            f.write_str("y")?;
        }
        if self.list {
            f.write_str("l")?;
        }
        if self.tag {
            f.write_str("t")?;
        }
        if self.move_blob {
            f.write_str("m")?;
        }
        if self.execute {
            f.write_str("e")?;
        }
        if self.ownership {
            f.write_str("o")?;
        }
        if self.permissions {
            f.write_str("p")?;
        }
        if self.set_immutability_policy {
            f.write_str("i")?;
        }
        Ok(())
    }
}

impl FromStr for BlobContainerSasPermissions {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut p = Self::default();
        for c in s.chars() {
            match c {
                'r' => p.read = true,
                'a' => p.add = true,
                'c' => p.create = true,
                'w' => p.write = true,
                'd' => p.delete = true,
                'x' => p.delete_version = true,
                'y' => p.permanent_delete = true,
                'l' => p.list = true,
                't' => p.tag = true,
                'm' => p.move_blob = true,
                'e' => p.execute = true,
                'o' => p.ownership = true,
                'p' => p.permissions = true,
                'i' => p.set_immutability_policy = true,
                other => {
                    return Err(Error::with_message(
                        ErrorKind::DataConversion,
                        format!("invalid blob container SAS permission character '{other}'"),
                    ));
                }
            }
        }
        Ok(p)
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
    delegated_user_object_id: Option<String>,
    cache_control: Option<String>,
    content_disposition: Option<String>,
    content_encoding: Option<String>,
    content_language: Option<String>,
    content_type: Option<String>,
    request_headers: Vec<(String, String)>,
    request_query_parameters: Vec<(String, String)>,
}

/// Canonicalize required-request headers for the `srh` STS slot. Format
/// matches the .NET storage SDK: `key1:value1\nkey2:value2\n` (with a
/// trailing newline), in the caller-supplied order. Returns an empty
/// string when there are no headers.
fn canonicalize_request_headers(headers: &[(String, String)]) -> String {
    let mut s = String::new();
    for (k, v) in headers {
        s.push_str(k);
        s.push(':');
        s.push_str(v);
        s.push('\n');
    }
    s
}

/// Canonicalize required-request query parameters for the `srq` STS slot.
/// Format matches the .NET storage SDK: `\nkey1:value1\nkey2:value2` (with
/// a *leading* newline before each pair), in the caller-supplied order.
/// Returns an empty string when there are no parameters.
fn canonicalize_request_query_parameters(params: &[(String, String)]) -> String {
    let mut s = String::new();
    for (k, v) in params {
        s.push('\n');
        s.push_str(k);
        s.push(':');
        s.push_str(v);
    }
    s
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
    /// Selects the SAS resource scope: blob, snapshot, or blob version.
    target: BlobTarget,
    state: S,
}

/// Identifies which blob-scoped resource a [`BlobSasBuilder`] targets.
///
/// Set indirectly via [`BlobSasBuilder::snapshot`] or
/// [`BlobSasBuilder::blob_version`].
#[derive(Clone, Debug, Default)]
enum BlobTarget {
    /// The blob itself (`sr=b`).
    #[default]
    Blob,
    /// A snapshot of the blob (`sr=bs`). Carries the snapshot identifier.
    Snapshot(String),
    /// A specific version of the blob (`sr=bv`). Carries the version id.
    Version(String),
}

impl BlobTarget {
    fn resource(&self) -> &'static str {
        match self {
            BlobTarget::Blob => "b",
            BlobTarget::Snapshot(_) => "bs",
            BlobTarget::Version(_) => "bv",
        }
    }

    fn signed_snapshot_time(&self) -> &str {
        match self {
            BlobTarget::Blob => "",
            BlobTarget::Snapshot(s) | BlobTarget::Version(s) => s.as_str(),
        }
    }
}

impl BlobSasBuilder<NoKey> {
    /// Create a new builder for the given blob with the required fields.
    ///
    /// `blob_name` must be the unencoded blob name. Backslashes are normalized
    /// to forward slashes, and any leading or trailing slashes are stripped to
    /// avoid producing a malformed canonical resource (for example, the path
    /// `/foo/bar` would otherwise yield `/blob/account/container//foo/bar`).
    pub fn new(
        container_name: String,
        blob_name: String,
        expiry: OffsetDateTime,
        permissions: BlobSasPermissions,
    ) -> Self {
        Self {
            container_name,
            blob_name: blob_name.replace('\\', "/").trim_matches('/').to_string(),
            expiry,
            permissions,
            common: CommonValues::default(),
            target: BlobTarget::Blob,
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
            target: self.target,
            state: WithKey(key),
        }
    }
}

impl<S> BlobSasBuilder<S> {
    common_setters!();

    /// Scope the SAS to a blob snapshot (`sr=bs`).
    ///
    /// `snapshot_id` is the snapshot identifier returned by the service
    /// (typically a timestamp such as `2025-01-02T03:04:05.6789012Z`).
    /// Mutually exclusive with [`blob_version`](Self::blob_version) — the
    /// last setter wins.
    pub fn snapshot(mut self, snapshot_id: impl Into<String>) -> Self {
        self.target = BlobTarget::Snapshot(snapshot_id.into());
        self
    }

    /// Scope the SAS to a specific blob version (`sr=bv`).
    ///
    /// `version_id` is the version identifier returned by the service.
    /// Mutually exclusive with [`snapshot`](Self::snapshot) — the last setter
    /// wins.
    pub fn blob_version(mut self, version_id: impl Into<String>) -> Self {
        self.target = BlobTarget::Version(version_id.into());
        self
    }
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
        validate_signing(&permissions, self.common.start, self.expiry)?;
        if self.blob_name.is_empty() {
            return Err(Error::with_message(
                ErrorKind::DataConversion,
                "blob_name is empty after normalization; use BlobContainerSasBuilder for a container-scoped SAS",
            ));
        }
        let canonical_resource = format!(
            "/blob/{}/{}/{}",
            account_name, self.container_name, self.blob_name
        );
        let resource = self.target.resource();
        let string_to_sign = build_string_to_sign(
            &permissions,
            self.common.start,
            self.expiry,
            &canonical_resource,
            key,
            &self.common,
            resource,
            self.target.signed_snapshot_time(),
        );

        let signature = sign(&string_to_sign, &key.value)?;

        let (snapshot, blob_version) = match self.target {
            BlobTarget::Blob => (None, None),
            BlobTarget::Snapshot(s) => (Some(s), None),
            BlobTarget::Version(v) => (None, Some(v)),
        };

        Ok(BlobSasQueryParameters {
            permissions,
            start: self.common.start,
            expiry: self.expiry,
            ip_range: self.common.ip_range,
            protocol: self.common.protocol,
            resource,
            key: key.clone(),
            correlation_id: self.common.correlation_id,
            preauthorized_agent_object_id: self.common.preauthorized_agent_object_id,
            agent_object_id: self.common.agent_object_id,
            delegated_user_object_id: self.common.delegated_user_object_id,
            encryption_scope: self.common.encryption_scope,
            cache_control: self.common.cache_control,
            content_disposition: self.common.content_disposition,
            content_encoding: self.common.content_encoding,
            content_language: self.common.content_language,
            content_type: self.common.content_type,
            request_headers: self.common.request_headers,
            request_query_parameters: self.common.request_query_parameters,
            signature,
            container_name: self.container_name,
            blob_name: Some(self.blob_name),
            snapshot,
            blob_version,
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
        validate_signing(&permissions, self.common.start, self.expiry)?;
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
            delegated_user_object_id: self.common.delegated_user_object_id,
            encryption_scope: self.common.encryption_scope,
            cache_control: self.common.cache_control,
            content_disposition: self.common.content_disposition,
            content_encoding: self.common.content_encoding,
            content_language: self.common.content_language,
            content_type: self.common.content_type,
            request_headers: self.common.request_headers,
            request_query_parameters: self.common.request_query_parameters,
            signature,
            container_name: self.container_name,
            blob_name: None,
            snapshot: None,
            blob_version: None,
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
        /// Set the AAD object id of the user being delegated to (emitted as `sduoid`).
        pub fn delegated_user_object_id(mut self, value: impl Into<String>) -> Self {
            self.common.delegated_user_object_id = Some(value.into());
            self
        }
        /// Require the caller to send the given HTTP request headers when
        /// using this SAS (signed in the `srh` slot; their names are emitted
        /// as the comma-joined `srh` query parameter). The order is
        /// preserved and is part of the signature — callers must replay it
        /// exactly. Replaces any previous value.
        pub fn request_headers<I, K, V>(mut self, headers: I) -> Self
        where
            I: IntoIterator<Item = (K, V)>,
            K: Into<String>,
            V: Into<String>,
        {
            self.common.request_headers = headers
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
            self
        }
        /// Require the caller to send the given URL query parameters when
        /// using this SAS (signed in the `srq` slot; their names are emitted
        /// as the comma-joined `srq` query parameter). The order is
        /// preserved and is part of the signature — callers must replay it
        /// exactly. Replaces any previous value.
        pub fn request_query_parameters<I, K, V>(mut self, params: I) -> Self
        where
            I: IntoIterator<Item = (K, V)>,
            K: Into<String>,
            V: Into<String>,
        {
            self.common.request_query_parameters = params
                .into_iter()
                .map(|(k, v)| (k.into(), v.into()))
                .collect();
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

fn validate_signing(
    permissions: &str,
    start: Option<OffsetDateTime>,
    expiry: OffsetDateTime,
) -> Result<()> {
    if permissions.is_empty() {
        return Err(Error::with_message(
            ErrorKind::DataConversion,
            "SAS permissions cannot be empty",
        ));
    }
    if let Some(start) = start {
        if expiry <= start {
            return Err(Error::with_message(
                ErrorKind::DataConversion,
                format!("SAS expiry ({expiry}) must be after start ({start})"),
            ));
        }
    }
    Ok(())
}

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

    let header_canon = canonicalize_request_headers(&common.request_headers);
    let query_canon = canonicalize_request_query_parameters(&common.request_query_parameters);

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
        key.signed_delegated_user_tenant_id.as_deref().unwrap_or(""),
        common.delegated_user_object_id.as_deref().unwrap_or(""),
        &ip,
        &protocol,
        SAS_VERSION,
        resource,
        signed_snapshot_time,
        common.encryption_scope.as_deref().unwrap_or(""),
        &header_canon,
        &query_canon,
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
    delegated_user_object_id: Option<String>,
    encryption_scope: Option<String>,
    cache_control: Option<String>,
    content_disposition: Option<String>,
    content_encoding: Option<String>,
    content_language: Option<String>,
    content_type: Option<String>,
    request_headers: Vec<(String, String)>,
    request_query_parameters: Vec<(String, String)>,
    signature: String,
    container_name: String,
    blob_name: Option<String>,
    snapshot: Option<String>,
    blob_version: Option<String>,
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

    /// The snapshot identifier this SAS targets, if any.
    pub fn snapshot(&self) -> Option<&str> {
        self.snapshot.as_deref()
    }

    /// The blob version this SAS targets, if any.
    pub fn blob_version(&self) -> Option<&str> {
        self.blob_version.as_deref()
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
        let prefix = match (&self.snapshot, &self.blob_version) {
            (Some(s), _) => format!("snapshot={}&", encode_query(s)),
            (_, Some(v)) => format!("versionid={}&", encode_query(v)),
            _ => String::new(),
        };
        Ok(format!("{}?{}{}", base, prefix, self))
    }
}

impl fmt::Display for BlobSasQueryParameters {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut first = true;
        fn write_pair(
            f: &mut fmt::Formatter<'_>,
            first: &mut bool,
            k: &str,
            v: &str,
        ) -> fmt::Result {
            if v.is_empty() {
                return Ok(());
            }
            if !*first {
                f.write_str("&")?;
            }
            *first = false;
            write!(f, "{}={}", k, encode_query(v))
        }

        write_pair(f, &mut first, "sv", SAS_VERSION)?;
        write_pair(f, &mut first, "sr", self.resource)?;
        if let Some(start) = self.start {
            write_pair(f, &mut first, "st", &format_sas_time(start))?;
        }
        write_pair(f, &mut first, "se", &format_sas_time(self.expiry))?;
        write_pair(f, &mut first, "sp", &self.permissions)?;
        if let Some(ip) = &self.ip_range {
            write_pair(f, &mut first, "sip", &ip.to_string())?;
        }
        if let Some(p) = self.protocol {
            write_pair(f, &mut first, "spr", p.as_str())?;
        }
        write_pair(f, &mut first, "skoid", &self.key.signed_object_id)?;
        write_pair(f, &mut first, "sktid", &self.key.signed_tenant_id)?;
        write_pair(
            f,
            &mut first,
            "skt",
            &format_sas_time(self.key.signed_start),
        )?;
        write_pair(
            f,
            &mut first,
            "ske",
            &format_sas_time(self.key.signed_expiry),
        )?;
        write_pair(f, &mut first, "sks", &self.key.signed_service)?;
        write_pair(f, &mut first, "skv", &self.key.signed_version)?;
        if let Some(v) = &self.preauthorized_agent_object_id {
            write_pair(f, &mut first, "saoid", v)?;
        }
        if let Some(v) = &self.agent_object_id {
            write_pair(f, &mut first, "suoid", v)?;
        }
        if let Some(v) = &self.key.signed_delegated_user_tenant_id {
            write_pair(f, &mut first, "skdutid", v)?;
        }
        if let Some(v) = &self.delegated_user_object_id {
            write_pair(f, &mut first, "sduoid", v)?;
        }
        if let Some(v) = &self.correlation_id {
            write_pair(f, &mut first, "scid", v)?;
        }
        if let Some(v) = &self.encryption_scope {
            write_pair(f, &mut first, "ses", v)?;
        }
        if !self.request_headers.is_empty() {
            let joined = self
                .request_headers
                .iter()
                .map(|(k, _)| encode_query(k))
                .collect::<Vec<_>>()
                .join(",");
            if !first {
                f.write_str("&")?;
            }
            first = false;
            write!(f, "srh={}", joined)?;
        }
        if !self.request_query_parameters.is_empty() {
            let joined = self
                .request_query_parameters
                .iter()
                .map(|(k, _)| encode_query(k))
                .collect::<Vec<_>>()
                .join(",");
            if !first {
                f.write_str("&")?;
            }
            first = false;
            write!(f, "srq={}", joined)?;
        }
        if let Some(v) = &self.cache_control {
            write_pair(f, &mut first, "rscc", v)?;
        }
        if let Some(v) = &self.content_disposition {
            write_pair(f, &mut first, "rscd", v)?;
        }
        if let Some(v) = &self.content_encoding {
            write_pair(f, &mut first, "rsce", v)?;
        }
        if let Some(v) = &self.content_language {
            write_pair(f, &mut first, "rscl", v)?;
        }
        if let Some(v) = &self.content_type {
            write_pair(f, &mut first, "rsct", v)?;
        }
        write_pair(f, &mut first, "sig", &self.signature)?;
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

    /// Locks in the positional layout of the user-delegation blob
    /// string-to-sign. The expected literal has 28 fields separated by 27
    /// newlines, matching the v2026-06-06 Azure spec.
    #[test]
    fn blob_string_to_sign_matches_expected_layout() {
        let perms = BlobSasPermissions {
            read: true,
            write: true,
            ..Default::default()
        };
        let common = CommonValues {
            start: Some(datetime!(2025-01-01 00:00:00 UTC)),
            protocol: Some(SasProtocol::Https),
            ip_range: Some(SasIpRange::single("10.0.0.1".parse().unwrap())),
            encryption_scope: Some("my-scope".into()),
            correlation_id: Some("abc-123".into()),
            preauthorized_agent_object_id: Some("saoid-value".into()),
            agent_object_id: Some("suoid-value".into()),
            delegated_user_object_id: None,
            cache_control: Some("no-cache".into()),
            content_disposition: Some("inline".into()),
            content_encoding: Some("gzip".into()),
            content_language: Some("en-US".into()),
            content_type: Some("text/plain".into()),
            ..Default::default()
        };
        let key = sample_key();
        let canonical = "/blob/myacct/c1/b1";
        let permissions = perms.to_permission_string();

        let actual = build_string_to_sign(
            &permissions,
            common.start,
            datetime!(2025-01-02 00:00:00 UTC),
            canonical,
            &key,
            &common,
            "b",
            "",
        );

        let expected = [
            "rw",                                   // signedPermissions
            "2025-01-01T00:00:00Z",                 // signedStart
            "2025-01-02T00:00:00Z",                 // signedExpiry
            "/blob/myacct/c1/b1",                   // canonicalizedResource
            "00000000-0000-0000-0000-000000000001", // signedKeyObjectId
            "00000000-0000-0000-0000-000000000002", // signedKeyTenantId
            "2025-01-01T00:00:00Z",                 // signedKeyStart
            "2025-01-08T00:00:00Z",                 // signedKeyExpiry
            "b",                                    // signedKeyService
            SAS_VERSION,                            // signedKeyVersion
            "saoid-value",                          // signedAuthorizedUserObjectId
            "suoid-value",                          // signedUnauthorizedUserObjectId
            "abc-123",                              // signedCorrelationId
            "",                                     // signedKeyDelegatedUserTenantId
            "",                                     // signedDelegatedUserObjectId
            "10.0.0.1",                             // signedIP
            "https",                                // signedProtocol
            SAS_VERSION,                            // signedVersion
            "b",                                    // signedResource
            "",                                     // signedSnapshotTime
            "my-scope",                             // signedEncryptionScope
            "",                                     // canonicalizedSignedRequestHeaders
            "",                                     // canonicalizedSignedRequestQueryParameters
            "no-cache",                             // rscc
            "inline",                               // rscd
            "gzip",                                 // rsce
            "en-US",                                // rscl
            "text/plain",                           // rsct
        ]
        .join("\n");

        assert_eq!(actual, expected);
        assert_eq!(actual.matches('\n').count(), 27);

        // The builder must sign exactly this string.
        let mut builder_common = common.clone();
        let _ = &mut builder_common; // keep clippy happy if unused below
        let sas = BlobSasBuilder {
            container_name: "c1".into(),
            blob_name: "b1".into(),
            expiry: datetime!(2025-01-02 00:00:00 UTC),
            permissions: perms,
            common,
            target: BlobTarget::Blob,
            state: WithKey(key.clone()),
        }
        .sign("myacct")
        .unwrap();

        let expected_sig = sign(&expected, &key.value).unwrap();
        assert_eq!(sas.signature(), expected_sig);
    }

    #[test]
    fn empty_optionals_still_produce_full_field_count() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let common = CommonValues::default();
        let key = sample_key();
        let sts = build_string_to_sign(
            &perms.to_permission_string(),
            None,
            datetime!(2025-01-02 00:00:00 UTC),
            "/blob/acct/c/b",
            &key,
            &common,
            "b",
            "",
        );
        // 28 fields => 27 separator newlines.
        assert_eq!(sts.matches('\n').count(), 27);
    }

    #[test]
    fn query_special_characters_are_percent_encoded() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .correlation_id("foo+bar=baz qux")
        .with_key(sample_key())
        .sign("acct")
        .unwrap();

        let query = sas.to_string();
        assert!(
            query.contains("&scid=foo%2Bbar%3Dbaz%20qux"),
            "query was: {query}"
        );
    }

    #[test]
    fn snapshot_sas_uses_bs_resource_and_signs_snapshot_time() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let snapshot_id = "2025-01-02T03:04:05.6789012Z";
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .snapshot(snapshot_id)
        .with_key(sample_key())
        .sign("acct")
        .unwrap();

        assert_eq!(sas.snapshot(), Some(snapshot_id));
        assert!(sas.to_string().contains("&sr=bs"));

        let url = sas.to_url("https://acct.blob.core.windows.net").unwrap();
        assert!(url.contains("/c/b?snapshot=2025-01-02T03:04:05.6789012Z&sv="));

        // The snapshot id must appear in the signed string.
        let expected_sts = build_string_to_sign(
            "r",
            None,
            datetime!(2025-01-02 00:00:00 UTC),
            "/blob/acct/c/b",
            &sample_key(),
            &CommonValues::default(),
            "bs",
            snapshot_id,
        );
        let expected_sig = sign(&expected_sts, &sample_key().value).unwrap();
        assert_eq!(sas.signature(), expected_sig);
    }

    #[test]
    fn version_sas_uses_bv_resource_and_versionid_param() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let version_id = "2025-01-02T03:04:05.6789012Z";
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .blob_version(version_id)
        .with_key(sample_key())
        .sign("acct")
        .unwrap();

        assert_eq!(sas.blob_version(), Some(version_id));
        assert!(sas.to_string().contains("&sr=bv"));
        let url = sas.to_url("https://acct.blob.core.windows.net").unwrap();
        assert!(url.contains("/c/b?versionid=2025-01-02T03:04:05.6789012Z&sv="));
    }

    #[test]
    fn snapshot_and_version_are_mutually_exclusive_last_wins() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .snapshot("s1")
        .blob_version("v1")
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        assert_eq!(sas.snapshot(), None);
        assert_eq!(sas.blob_version(), Some("v1"));
    }

    #[test]
    fn delegated_user_ids_render_and_sign() {
        let perms = BlobSasPermissions {
            read: true,
            ..Default::default()
        };
        let key = sample_key().with_signed_delegated_user_tenant_id("tenant-123");
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            perms,
        )
        .delegated_user_object_id("user-456")
        .with_key(key.clone())
        .sign("acct")
        .unwrap();

        let query = sas.to_string();
        assert!(query.contains("&skdutid=tenant-123"), "query: {query}");
        assert!(query.contains("&sduoid=user-456"), "query: {query}");

        let mut common = CommonValues::default();
        common.delegated_user_object_id = Some("user-456".into());
        let expected_sts = build_string_to_sign(
            "r",
            None,
            datetime!(2025-01-02 00:00:00 UTC),
            "/blob/acct/c/b",
            &key,
            &common,
            "b",
            "",
        );
        let expected_sig = sign(&expected_sts, &key.value).unwrap();
        assert_eq!(sas.signature(), expected_sig);
        // The IDs must appear in the signed string (positions 13 and 14, 0-indexed).
        let parts: Vec<&str> = expected_sts.split('\n').collect();
        assert_eq!(parts[13], "tenant-123");
        assert_eq!(parts[14], "user-456");
    }

    #[test]
    fn blob_permissions_all_and_read_only() {
        assert_eq!(BlobSasPermissions::all().to_string(), "racwdxytmeopi");
        assert_eq!(BlobSasPermissions::read_only().to_string(), "r");
    }

    #[test]
    fn container_permissions_all_and_read_only() {
        assert_eq!(
            BlobContainerSasPermissions::all().to_string(),
            "racwdxyltmeopi"
        );
        assert_eq!(BlobContainerSasPermissions::read_only().to_string(), "rl");
    }

    #[test]
    fn blob_permissions_round_trip() {
        let p = BlobSasPermissions::all();
        let parsed: BlobSasPermissions = p.to_string().parse().unwrap();
        assert_eq!(parsed, p);
    }

    #[test]
    fn container_permissions_round_trip() {
        let p = BlobContainerSasPermissions::all();
        let parsed: BlobContainerSasPermissions = p.to_string().parse().unwrap();
        assert_eq!(parsed, p);
    }

    #[test]
    fn permissions_parse_normalizes_order() {
        // Input letters may be in any order; the canonical Display order is fixed.
        let p: BlobSasPermissions = "dwr".parse().unwrap();
        assert_eq!(p.to_string(), "rwd");
    }

    #[test]
    fn permissions_parse_rejects_unknown_letter() {
        let err = "rz".parse::<BlobSasPermissions>().unwrap_err();
        assert!(err.to_string().contains("invalid"), "got: {err}");
    }

    #[test]
    fn sign_rejects_empty_permissions() {
        let err = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            BlobSasPermissions::default(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap_err();
        assert!(err.to_string().contains("cannot be empty"), "got: {err}");
    }

    #[test]
    fn sign_rejects_expiry_before_start() {
        let err = BlobSasBuilder::new(
            "c".into(),
            "b".into(),
            datetime!(2025-01-01 00:00:00 UTC),
            BlobSasPermissions::read_only(),
        )
        .start(datetime!(2025-01-02 00:00:00 UTC))
        .with_key(sample_key())
        .sign("acct")
        .unwrap_err();
        assert!(
            err.to_string().contains("must be after start"),
            "got: {err}"
        );
    }

    #[test]
    fn container_sas_never_emits_snapshot_or_versionid() {
        let sas = BlobContainerSasBuilder::new(
            "c".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            BlobContainerSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        let url = sas.to_url("https://acct.blob.core.windows.net").unwrap();
        assert!(!url.contains("snapshot="));
        assert!(!url.contains("versionid="));
        assert_eq!(sas.snapshot(), None);
        assert_eq!(sas.blob_version(), None);
    }

    #[test]
    fn sign_rejects_empty_blob_name() {
        let err = BlobSasBuilder::new(
            "c".into(),
            "/".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            BlobSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap_err();
        assert!(
            err.to_string().contains("BlobContainerSasBuilder"),
            "got: {err}"
        );
    }

    #[test]
    fn request_headers_and_query_parameters_canonicalize_and_render() {
        let common = CommonValues {
            request_headers: vec![
                ("x-ms-foo".into(), "1".into()),
                ("x-ms-bar".into(), "two".into()),
            ],
            request_query_parameters: vec![
                ("alpha".into(), "a".into()),
                ("beta".into(), "b".into()),
            ],
            ..Default::default()
        };
        let sts = build_string_to_sign(
            "r",
            None,
            datetime!(2025-01-02 00:00:00 UTC),
            "/blob/acct/c/b",
            &sample_key(),
            &common,
            "b",
            "",
        );
        // Headers canonical form: "k:v\nk:v\n" at field index 21.
        let parts: Vec<&str> = sts.split('\n').collect();
        // Field 21 begins the multi-line header block. Locate it by finding the
        // first line equal to the encryption-scope (empty) + headers run.
        assert!(
            sts.contains("x-ms-foo:1\nx-ms-bar:two\n"),
            "sts missing canonical headers: {sts}"
        );
        // Query-parameter canonical run is "\nk:v\nk:v" (leading newline).
        assert!(
            sts.contains("\nalpha:a\nbeta:b"),
            "sts missing canonical query params: {sts}"
        );
        // Compile-only sanity that we still parse to the right number of pieces.
        assert!(parts.len() >= 28);
    }

    #[test]
    fn sas_display_emits_srh_and_srq_keys() {
        let sas = BlobSasBuilder::new(
            "c".into(),
            "b.txt".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            BlobSasPermissions::read_only(),
        )
        .request_headers([("x-ms-foo", "1"), ("x-ms-bar", "two")])
        .request_query_parameters([("alpha", "a"), ("beta", "b")])
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        let rendered = sas.to_string();
        assert!(
            rendered.contains("srh=x-ms-foo%2Cx-ms-bar")
                || rendered.contains("srh=x-ms-foo,x-ms-bar"),
            "got: {rendered}"
        );
        assert!(
            rendered.contains("srq=alpha,beta") || rendered.contains("srq=alpha%2Cbeta"),
            "got: {rendered}"
        );
    }
}
