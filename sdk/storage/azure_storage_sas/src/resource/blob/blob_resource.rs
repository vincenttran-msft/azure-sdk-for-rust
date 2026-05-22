// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::builder::Fields;
use crate::resource::blob::{blob_udk_query_parameters, blob_udk_string_to_sign};
use crate::resource::{sealed, BlobServiceResource, Resource};
use crate::UserDelegationKey;
use std::fmt;

/// A blob resource for user delegation SAS.
///
/// By default targets a base blob (`sr=b`). Use [`Blob::snapshot`] or
/// [`Blob::version`] to target a snapshot (`sr=bs`) or version (`sr=bv`).
pub struct Blob {
    container: String,
    blob: String,
    snapshot: Option<String>,
    version_id: Option<String>,
}

impl Blob {
    /// Creates a new blob resource targeting the base blob.
    pub fn new(container: impl Into<String>, blob: impl Into<String>) -> Self {
        Self {
            container: container.into(),
            blob: blob.into(),
            snapshot: None,
            version_id: None,
        }
    }

    /// Targets a specific snapshot of the blob (`sr=bs`).
    ///
    /// `snapshot` is the snapshot timestamp (e.g., `"2025-01-15T12:00:00.0000000Z"`).
    pub fn snapshot(mut self, snapshot: impl Into<String>) -> Self {
        self.snapshot = Some(snapshot.into());
        self
    }

    /// Targets a specific version of the blob (`sr=bv`).
    ///
    /// The version ID is not included in the SAS token itself — the caller
    /// appends `&versionid=...` to the request URL separately.
    pub fn version(mut self, version_id: impl Into<String>) -> Self {
        self.version_id = Some(version_id.into());
        self
    }

    fn signed_resource(&self) -> &'static str {
        if self.snapshot.is_some() {
            "bs"
        } else if self.version_id.is_some() {
            "bv"
        } else {
            "b"
        }
    }

    fn canonicalized_resource(&self, account: &str) -> String {
        format!("/blob/{}/{}/{}", account, self.container, self.blob)
    }
}

/// Permissions for a blob SAS.
///
/// Serialization order: `racwdxytmeopi`.
#[derive(Clone, Copy, Default)]
pub struct BlobPermissions {
    pub read: bool,
    pub add: bool,
    pub create: bool,
    pub write: bool,
    pub delete: bool,
    pub delete_version: bool,
    pub permanent_delete: bool,
    pub tags: bool,
    pub move_blob: bool,
    pub execute: bool,
    pub ownership: bool,
    pub permissions: bool,
    pub set_immutability_policy: bool,
}

impl fmt::Display for BlobPermissions {
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
        if self.tags {
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

impl sealed::Sealed for Blob {}
impl BlobServiceResource for Blob {}

impl Resource for Blob {
    type Permissions = BlobPermissions;

    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        key: &UserDelegationKey,
    ) -> String {
        blob_udk_string_to_sign(
            permissions,
            fields,
            key,
            self.signed_resource(),
            &self.canonicalized_resource(&fields.account),
            self.snapshot.as_deref().unwrap_or(""),
            "",
        )
    }

    fn _build_query_parameters(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        key: &UserDelegationKey,
        signature: &str,
    ) -> String {
        blob_udk_query_parameters(
            permissions,
            fields,
            key,
            self.signed_resource(),
            self.snapshot.as_deref(),
            None,
            signature,
        )
    }
}
