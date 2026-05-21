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

/// Permissions that can be granted by a SAS for a single queue.
///
/// Letters are emitted in the canonical Azure order: `raup`.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct QueueSasPermissions {
    /// Read metadata and properties, peek and retrieve messages.
    pub read: bool,
    /// Add messages to the queue.
    pub add: bool,
    /// Update an existing message.
    pub update: bool,
    /// Get and delete messages (the "process" permission).
    pub process: bool,
}

impl QueueSasPermissions {
    /// All defined queue permissions set to `true`.
    pub fn all() -> Self {
        Self {
            read: true,
            add: true,
            update: true,
            process: true,
        }
    }

    /// Convenience constructor for read-only queue access.
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

impl fmt::Display for QueueSasPermissions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.read {
            f.write_str("r")?;
        }
        if self.add {
            f.write_str("a")?;
        }
        if self.update {
            f.write_str("u")?;
        }
        if self.process {
            f.write_str("p")?;
        }
        Ok(())
    }
}

impl FromStr for QueueSasPermissions {
    type Err = Error;

    fn from_str(s: &str) -> Result<Self> {
        let mut perms = Self::default();
        for c in s.chars() {
            match c {
                'r' => perms.read = true,
                'a' => perms.add = true,
                'u' => perms.update = true,
                'p' => perms.process = true,
                other => {
                    return Err(Error::with_message_fn(
                        ErrorKind::DataConversion,
                        move || format!("invalid queue SAS permission character: {other:?}"),
                    ))
                }
            }
        }
        Ok(perms)
    }
}

// ---------------------------------------------------------------------------
// Builder
// ---------------------------------------------------------------------------

/// Builder for a user delegation SAS scoped to a single queue.
///
/// Construct with [`QueueSasBuilder::new`], attach a
/// [`UserDelegationKey`] with [`with_key`](QueueSasBuilder::with_key), then
/// call [`sign`](QueueSasBuilder::sign) to produce a
/// [`QueueSasQueryParameters`].
#[derive(Clone, Debug)]
pub struct QueueSasBuilder<S = NoKey> {
    queue_name: String,
    expiry: OffsetDateTime,
    permissions: QueueSasPermissions,
    start: Option<OffsetDateTime>,
    protocol: Option<SasProtocol>,
    ip_range: Option<SasIpRange>,
    delegated_user_object_id: Option<String>,
    state: S,
}

impl QueueSasBuilder<NoKey> {
    /// Create a new builder for the given queue with the required fields.
    pub fn new(
        queue_name: String,
        expiry: OffsetDateTime,
        permissions: QueueSasPermissions,
    ) -> Self {
        Self {
            queue_name,
            expiry,
            permissions,
            start: None,
            protocol: None,
            ip_range: None,
            delegated_user_object_id: None,
            state: NoKey,
        }
    }

    /// Attach the user delegation key obtained from the service.
    pub fn with_key(self, key: UserDelegationKey) -> QueueSasBuilder<WithKey> {
        QueueSasBuilder {
            queue_name: self.queue_name,
            expiry: self.expiry,
            permissions: self.permissions,
            start: self.start,
            protocol: self.protocol,
            ip_range: self.ip_range,
            delegated_user_object_id: self.delegated_user_object_id,
            state: WithKey(key),
        }
    }
}

impl<S> QueueSasBuilder<S> {
    /// Set the SAS start time (emitted as `st`).
    pub fn start(mut self, start: OffsetDateTime) -> Self {
        self.start = Some(start);
        self
    }

    /// Restrict the SAS to a specific protocol (emitted as `spr`).
    pub fn protocol(mut self, protocol: SasProtocol) -> Self {
        self.protocol = Some(protocol);
        self
    }

    /// Restrict the SAS to a single IP address or range (emitted as `sip`).
    pub fn ip_range(mut self, ip_range: SasIpRange) -> Self {
        self.ip_range = Some(ip_range);
        self
    }

    /// Set the AAD object id of the user being delegated to (emitted as `sduoid`).
    pub fn delegated_user_object_id(mut self, value: impl Into<String>) -> Self {
        self.delegated_user_object_id = Some(value.into());
        self
    }
}

impl QueueSasBuilder<WithKey> {
    /// Produce the signed SAS query parameters for the given storage account.
    pub fn sign(self, account_name: &str) -> Result<QueueSasQueryParameters> {
        if self.queue_name.is_empty() {
            return Err(Error::with_message(
                ErrorKind::DataConversion,
                "queue_name must not be empty",
            ));
        }
        if account_name.is_empty() {
            return Err(Error::with_message(
                ErrorKind::DataConversion,
                "account_name must not be empty",
            ));
        }

        let key = &self.state.0;
        let permissions = self.permissions.to_permission_string();
        if permissions.is_empty() {
            return Err(Error::with_message(
                ErrorKind::DataConversion,
                "at least one permission must be set",
            ));
        }
        if let Some(start) = self.start {
            if self.expiry <= start {
                let expiry = self.expiry;
                return Err(Error::with_message_fn(
                    ErrorKind::DataConversion,
                    move || format!("SAS expiry ({}) must be after start ({})", expiry, start),
                ));
            }
        }
        let canonical_resource = format!("/queue/{}/{}", account_name, self.queue_name);

        let string_to_sign = build_string_to_sign(
            &permissions,
            self.start,
            self.expiry,
            &canonical_resource,
            key,
            self.ip_range.as_ref(),
            self.protocol,
            self.delegated_user_object_id.as_deref(),
        );
        let signature = sign(&string_to_sign, &key.value)?;

        Ok(QueueSasQueryParameters {
            permissions,
            start: self.start,
            expiry: self.expiry,
            ip_range: self.ip_range,
            protocol: self.protocol,
            key: key.clone(),
            delegated_user_object_id: self.delegated_user_object_id,
            signature,
            queue_name: self.queue_name,
        })
    }
}

// ---------------------------------------------------------------------------
// String-to-sign
// ---------------------------------------------------------------------------

/// Build the 15-field user delegation SAS string-to-sign for a queue.
///
/// Layout (one field per line, in this order):
/// 1. signedPermissions
/// 2. signedStart
/// 3. signedExpiry
/// 4. canonicalizedResource (`/queue/{account}/{queue}`)
/// 5. signedKeyObjectId
/// 6. signedKeyTenantId
/// 7. signedKeyStart
/// 8. signedKeyExpiry
/// 9. signedKeyService
/// 10. signedKeyVersion
/// 11. signedKeyDelegatedUserTenantId
/// 12. signedDelegatedUserObjectId
/// 13. signedIP
/// 14. signedProtocol
/// 15. signedVersion
#[allow(clippy::too_many_arguments)]
fn build_string_to_sign(
    permissions: &str,
    start: Option<OffsetDateTime>,
    expiry: OffsetDateTime,
    canonical_resource: &str,
    key: &UserDelegationKey,
    ip_range: Option<&SasIpRange>,
    protocol: Option<SasProtocol>,
    delegated_user_object_id: Option<&str>,
) -> String {
    let start_str = start.map(format_sas_time).unwrap_or_default();
    let expiry_str = format_sas_time(expiry);
    let key_start = format_sas_time(key.signed_start);
    let key_expiry = format_sas_time(key.signed_expiry);
    let ip = ip_range.map(|r| r.to_string()).unwrap_or_default();
    let protocol_str = protocol.map(|p| p.as_str().to_string()).unwrap_or_default();

    [
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
        key.signed_delegated_user_tenant_id.as_deref().unwrap_or(""),
        delegated_user_object_id.unwrap_or(""),
        &ip,
        &protocol_str,
        SAS_VERSION,
    ]
    .join("\n")
}

// ---------------------------------------------------------------------------
// Query parameters
// ---------------------------------------------------------------------------

/// A signed user delegation SAS for a queue, ready to be appended to a URL.
#[derive(Clone, Debug)]
pub struct QueueSasQueryParameters {
    permissions: String,
    start: Option<OffsetDateTime>,
    expiry: OffsetDateTime,
    ip_range: Option<SasIpRange>,
    protocol: Option<SasProtocol>,
    key: UserDelegationKey,
    delegated_user_object_id: Option<String>,
    signature: String,
    queue_name: String,
}

impl QueueSasQueryParameters {
    /// The signed permissions string (the `sp` value).
    pub fn permissions(&self) -> &str {
        &self.permissions
    }

    /// The queue that the SAS targets.
    pub fn queue_name(&self) -> &str {
        &self.queue_name
    }

    /// The computed signature (the `sig` value, base64 encoded).
    pub fn signature(&self) -> &str {
        &self.signature
    }

    /// Combine the SAS query string with the given storage endpoint
    /// (e.g. `https://myaccount.queue.core.windows.net`) and the queue name
    /// to produce a complete URL.
    pub fn to_url(&self, endpoint: &str) -> Result<String> {
        let base = append_path(endpoint, &self.queue_name);
        Ok(format!("{}?{}", base, self))
    }
}

impl fmt::Display for QueueSasQueryParameters {
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
        if let Some(v) = &self.key.signed_delegated_user_tenant_id {
            write_pair(f, "skdutid", v)?;
        }
        if let Some(v) = &self.delegated_user_object_id {
            write_pair(f, "sduoid", v)?;
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
            "q",
            SAS_VERSION,
            "dGVzdC1rZXk=", // "test-key"
        )
    }

    #[test]
    fn permissions_all_and_read_only() {
        assert_eq!(QueueSasPermissions::all().to_string(), "raup");
        assert_eq!(QueueSasPermissions::read_only().to_string(), "r");
    }

    #[test]
    fn permissions_round_trip_via_from_str() {
        let parsed: QueueSasPermissions = "raup".parse().unwrap();
        assert_eq!(parsed, QueueSasPermissions::all());
    }

    #[test]
    fn permissions_reject_invalid_letter() {
        let err = "rax".parse::<QueueSasPermissions>().unwrap_err();
        assert!(format!("{err}").contains("invalid queue SAS permission character"));
    }

    /// Locks in the positional layout of the user-delegation queue
    /// string-to-sign: 15 fields, 14 newlines.
    #[test]
    fn queue_string_to_sign_matches_expected_layout() {
        let key = sample_key().with_signed_delegated_user_tenant_id("tenant-x");
        let perms = QueueSasPermissions::all();
        let permissions = perms.to_permission_string();
        let ip = SasIpRange::single("10.0.0.1".parse().unwrap());

        let actual = build_string_to_sign(
            &permissions,
            Some(datetime!(2025-01-01 00:00:00 UTC)),
            datetime!(2025-01-02 00:00:00 UTC),
            "/queue/acct/myqueue",
            &key,
            Some(&ip),
            Some(SasProtocol::Https),
            Some("user-1"),
        );

        let expected = [
            "raup",                                 // signedPermissions
            "2025-01-01T00:00:00Z",                 // signedStart
            "2025-01-02T00:00:00Z",                 // signedExpiry
            "/queue/acct/myqueue",                  // canonicalizedResource
            "00000000-0000-0000-0000-000000000001", // signedKeyObjectId
            "00000000-0000-0000-0000-000000000002", // signedKeyTenantId
            "2025-01-01T00:00:00Z",                 // signedKeyStart
            "2025-01-08T00:00:00Z",                 // signedKeyExpiry
            "q",                                    // signedKeyService
            SAS_VERSION,                            // signedKeyVersion
            "tenant-x",                             // signedKeyDelegatedUserTenantId
            "user-1",                               // signedDelegatedUserObjectId
            "10.0.0.1",                             // signedIP
            "https",                                // signedProtocol
            SAS_VERSION,                            // signedVersion
        ]
        .join("\n");

        assert_eq!(actual, expected);
        assert_eq!(actual.matches('\n').count(), 14);

        let sas = QueueSasBuilder::new("myqueue".into(), datetime!(2025-01-02 00:00:00 UTC), perms)
            .start(datetime!(2025-01-01 00:00:00 UTC))
            .protocol(SasProtocol::Https)
            .ip_range(ip)
            .delegated_user_object_id("user-1")
            .with_key(key.clone())
            .sign("acct")
            .unwrap();

        let expected_sig = sign(&expected, &key.value).unwrap();
        assert_eq!(sas.signature(), expected_sig);
    }

    #[test]
    fn empty_optionals_still_produce_full_field_count() {
        let key = sample_key();
        let sts = build_string_to_sign(
            "r",
            None,
            datetime!(2025-01-02 00:00:00 UTC),
            "/queue/acct/q",
            &key,
            None,
            None,
            None,
        );
        // 15 fields => 14 separator newlines.
        assert_eq!(sts.matches('\n').count(), 14);
    }

    #[test]
    fn sign_rejects_empty_queue_name() {
        let err = QueueSasBuilder::new(
            String::new(),
            datetime!(2025-01-02 00:00:00 UTC),
            QueueSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap_err();
        assert!(format!("{err}").contains("queue_name"));
    }

    #[test]
    fn sign_rejects_empty_account_name() {
        let err = QueueSasBuilder::new(
            "q".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            QueueSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("")
        .unwrap_err();
        assert!(format!("{err}").contains("account_name"));
    }

    #[test]
    fn sign_rejects_no_permissions() {
        let err = QueueSasBuilder::new(
            "q".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            QueueSasPermissions::default(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap_err();
        assert!(format!("{err}").contains("permission"));
    }

    #[test]
    fn renders_query_string_in_canonical_order() {
        let sas = QueueSasBuilder::new(
            "q".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            QueueSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        let s = sas.to_string();
        // sv comes first, sig comes last.
        assert!(s.starts_with(&format!("sv={SAS_VERSION}")), "{s}");
        assert!(s.contains("&sig="), "{s}");
    }

    #[test]
    fn to_url_appends_query() {
        let sas = QueueSasBuilder::new(
            "myqueue".into(),
            datetime!(2025-01-02 00:00:00 UTC),
            QueueSasPermissions::read_only(),
        )
        .with_key(sample_key())
        .sign("acct")
        .unwrap();
        let url = sas.to_url("https://acct.queue.core.windows.net").unwrap();
        assert!(
            url.starts_with("https://acct.queue.core.windows.net/myqueue?sv="),
            "{url}"
        );
    }

    #[test]
    fn sign_rejects_expiry_before_start() {
        let err = QueueSasBuilder::new(
            "myqueue".into(),
            datetime!(2025-01-01 00:00:00 UTC),
            QueueSasPermissions::read_only(),
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
}
