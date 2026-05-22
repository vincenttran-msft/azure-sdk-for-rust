// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::builder::Fields;
use crate::resource::blob::{blob_udk_query_parameters, blob_udk_string_to_sign};
use crate::resource::{sealed, BlobServiceResource, DelegatedResource, Resource};
use crate::UserDelegationKey;
use std::fmt;

/// A container resource for user delegation SAS.
pub struct Container {
    container: String,
}

impl Container {
    /// Creates a new container resource.
    pub fn new(container: impl Into<String>) -> Self {
        Self {
            container: container.into(),
        }
    }

    fn canonicalized_resource(&self, account: &str) -> String {
        format!("/blob/{}/{}", account, self.container)
    }
}

/// Permissions for a container or directory SAS.
///
/// Serialization order: `racwdxyltmeopi`.
#[derive(Clone, Copy, Default)]
pub struct ContainerPermissions {
    pub read: bool,
    pub add: bool,
    pub create: bool,
    pub write: bool,
    pub delete: bool,
    pub delete_version: bool,
    pub permanent_delete: bool,
    pub list: bool,
    pub tags: bool,
    pub move_blob: bool,
    pub execute: bool,
    pub ownership: bool,
    pub permissions: bool,
    pub set_immutability_policy: bool,
}

impl fmt::Display for ContainerPermissions {
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

impl sealed::Sealed for Container {}
impl BlobServiceResource for Container {}
impl DelegatedResource for Container {}

impl Resource for Container {
    type Permissions = ContainerPermissions;
    type SigningContext = UserDelegationKey;

    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
    ) -> String {
        blob_udk_string_to_sign(
            permissions,
            fields,
            context,
            "c",
            &self.canonicalized_resource(&fields.account),
            "",
            "",
        )
    }

    fn _build_query_parameters(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
        signature: &str,
    ) -> String {
        blob_udk_query_parameters(permissions, fields, context, "c", None, None, signature)
    }
}
