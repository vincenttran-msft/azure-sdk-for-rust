// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::generated::models::UserDelegationKey;

impl From<UserDelegationKey> for azure_storage_sas::UserDelegationKey {
    fn from(key: UserDelegationKey) -> Self {
        Self {
            signed_oid: key.signed_oid.unwrap_or_default(),
            signed_tid: key.signed_tid.unwrap_or_default(),
            signed_start: key.signed_start.expect("signed_start is required"),
            signed_expiry: key.signed_expiry.expect("signed_expiry is required"),
            signed_service: key.signed_service.unwrap_or_default(),
            signed_version: key.signed_version.unwrap_or_default(),
            value: key.value.unwrap_or_default(),
        }
    }
}
