// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::fmt::SafeDebug;
use serde::{Deserialize, Serialize};

/// A user delegation key returned by
/// [`BlobServiceClient::get_user_delegation_key`](crate::BlobServiceClient::get_user_delegation_key).
///
/// The secret material (`value`) signs user delegation SAS tokens. It is
/// hidden from the [`Debug`] representation via [`SafeDebug`].
#[derive(Clone, Default, Deserialize, SafeDebug, Serialize)]
#[non_exhaustive]
#[serde(rename = "UserDelegationKey")]
pub struct UserDelegationKey {
    /// The Azure Active Directory object ID associated with the key.
    #[serde(rename = "SignedOid")]
    pub signed_object_id: String,

    /// The Azure Active Directory tenant ID associated with the key.
    #[serde(rename = "SignedTid")]
    pub signed_tenant_id: String,

    /// The time at which the key becomes valid, as an RFC 3339 string.
    #[serde(rename = "SignedStart")]
    pub signed_start: String,

    /// The time at which the key expires, as an RFC 3339 string.
    #[serde(rename = "SignedExpiry")]
    pub signed_expiry: String,

    /// The storage service identifier (for example, `"b"` for Blob).
    #[serde(rename = "SignedService")]
    pub signed_service: String,

    /// The service version used to issue the key.
    #[serde(rename = "SignedVersion")]
    pub signed_version: String,

    /// The AAD tenant ID of the end user this key was delegated to, if the
    /// request asked for a user-bound key.
    #[serde(
        rename = "SignedDelegatedUserTid",
        skip_serializing_if = "Option::is_none"
    )]
    pub signed_delegated_user_tenant_id: Option<String>,

    /// The base64-encoded secret key value.
    #[safe(false)]
    #[serde(rename = "Value")]
    pub value: String,
}

/// Request body for `Get User Delegation Key`. Internal: serialized as the
/// XML root element `KeyInfo`.
#[derive(Serialize)]
#[serde(rename = "KeyInfo")]
pub(crate) struct KeyInfo {
    #[serde(rename = "Start")]
    pub start: String,
    #[serde(rename = "Expiry")]
    pub expiry: String,
}
