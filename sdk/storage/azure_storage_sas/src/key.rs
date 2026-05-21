// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::fmt::SafeDebug;
use time::OffsetDateTime;

/// A user delegation key obtained from a storage service.
///
/// Obtain one by calling `get_user_delegation_key` on a Storage service client
/// (for example, `BlobServiceClient`). The key contains the secret material
/// used to sign user delegation SAS tokens.
///
/// The secret value (`value`) is intentionally hidden from the [`Debug`]
/// representation via [`SafeDebug`].
#[derive(Clone, SafeDebug)]
pub struct UserDelegationKey {
    /// The Azure Active Directory object ID associated with the key.
    pub signed_object_id: String,
    /// The Azure Active Directory tenant ID associated with the key.
    pub signed_tenant_id: String,
    /// The time at which the key becomes valid.
    pub signed_start: OffsetDateTime,
    /// The time at which the key expires.
    pub signed_expiry: OffsetDateTime,
    /// The storage service identifier (for example, `"b"` for Blob).
    pub signed_service: String,
    /// The service version used to issue the key.
    pub signed_version: String,
    /// The base64-encoded secret key value.
    #[safe(false)]
    pub value: String,
}

impl UserDelegationKey {
    /// Create a new user delegation key from its constituent fields.
    pub fn new(
        signed_object_id: impl Into<String>,
        signed_tenant_id: impl Into<String>,
        signed_start: OffsetDateTime,
        signed_expiry: OffsetDateTime,
        signed_service: impl Into<String>,
        signed_version: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            signed_object_id: signed_object_id.into(),
            signed_tenant_id: signed_tenant_id.into(),
            signed_start,
            signed_expiry,
            signed_service: signed_service.into(),
            signed_version: signed_version.into(),
            value: value.into(),
        }
    }
}
