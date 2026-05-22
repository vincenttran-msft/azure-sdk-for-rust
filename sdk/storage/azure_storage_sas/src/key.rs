// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

/// A user delegation key obtained from the Blob service, used to sign
/// user delegation SAS tokens.
///
/// All string fields should contain the values exactly as returned by the
/// [Get User Delegation Key](https://learn.microsoft.com/rest/api/storageservices/get-user-delegation-key)
/// REST API.
#[derive(Clone)]
pub struct UserDelegationKey {
    /// The object ID of the key's owner (skoid).
    pub signed_oid: String,
    /// The tenant ID of the key's owner (sktid).
    pub signed_tid: String,
    /// The start time of the key's validity period (skt), in ISO 8601 format.
    pub signed_start: String,
    /// The expiry time of the key's validity period (ske), in ISO 8601 format.
    pub signed_expiry: String,
    /// The service the key is scoped to (sks), typically "b" for blob.
    pub signed_service: String,
    /// The service version used to obtain the key (skv).
    pub signed_version: String,
    /// The base64-encoded key value used for HMAC signing.
    pub value: String,
}
