// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::builder::Fields;
use crate::resource::blob::container::ContainerPermissions;
use crate::resource::blob::{blob_udk_query_parameters, blob_udk_string_to_sign};
use crate::resource::{sealed, BlobServiceResource, Resource};
use crate::UserDelegationKey;

/// A directory resource (ADLS Gen2) for user delegation SAS.
pub struct Directory {
    container: String,
    directory: String,
}

impl Directory {
    /// Creates a new directory resource.
    ///
    /// The directory depth (`sdd`) is computed automatically from the path
    /// by counting `/`-separated segments (e.g., `"dir1/dir2"` → depth 2).
    pub fn new(container: impl Into<String>, directory: impl Into<String>) -> Self {
        Self {
            container: container.into(),
            directory: directory.into(),
        }
    }

    fn depth(&self) -> u32 {
        let trimmed = self.directory.trim_matches('/');
        if trimmed.is_empty() {
            0
        } else {
            trimmed.split('/').count() as u32
        }
    }

    fn canonicalized_resource(&self, account: &str) -> String {
        format!("/blob/{}/{}/{}", account, self.container, self.directory)
    }
}

impl sealed::Sealed for Directory {}
impl BlobServiceResource for Directory {}

impl Resource for Directory {
    type Permissions = ContainerPermissions;

    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        key: &UserDelegationKey,
    ) -> String {
        let depth_str = self.depth().to_string();
        blob_udk_string_to_sign(
            permissions,
            fields,
            key,
            "d",
            &self.canonicalized_resource(&fields.account),
            "",
            &depth_str,
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
            "d",
            None,
            Some(self.depth()),
            signature,
        )
    }
}
