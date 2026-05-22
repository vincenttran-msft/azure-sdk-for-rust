// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use time::OffsetDateTime;

/// A user delegation key obtained from the Blob service, used to sign
/// user delegation SAS tokens.
///
/// Field values should match the
/// [Get User Delegation Key](https://learn.microsoft.com/rest/api/storageservices/get-user-delegation-key)
/// REST API response.
#[derive(Clone)]
pub struct UserDelegationKey {
    /// The object ID of the key's owner (skoid).
    pub signed_oid: String,
    /// The tenant ID of the key's owner (sktid).
    pub signed_tid: String,
    /// The start time of the key's validity period (skt).
    pub signed_start: OffsetDateTime,
    /// The expiry time of the key's validity period (ske).
    pub signed_expiry: OffsetDateTime,
    /// The service the key is scoped to (sks), typically "b" for blob.
    pub signed_service: String,
    /// The service version used to obtain the key (skv).
    pub signed_version: String,
    /// The decoded key bytes used for HMAC-SHA256 signing.
    pub value: Vec<u8>,
}
