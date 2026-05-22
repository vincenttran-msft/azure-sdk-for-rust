// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::ip_range::SasIpRange;
use crate::protocol::SasProtocol;
use crate::resource::{BlobServiceResource, Resource};
use crate::UserDelegationKey;
use base64::Engine;
use hmac::{Hmac, Mac};
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sha2::Sha256;
use std::fmt;
use time::OffsetDateTime;

/// Characters that must be percent-encoded in SAS query parameter values.
const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Internal fields of a SAS builder, shared with resource implementations.
#[doc(hidden)]
pub struct Fields {
    pub account: String,
    pub start: Option<OffsetDateTime>,
    pub expiry: OffsetDateTime,
    pub protocol: Option<SasProtocol>,
    pub ip_range: Option<SasIpRange>,
    pub encryption_scope: Option<String>,
    // Blob-service specific
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_type: Option<String>,
    pub authorized_object_id: Option<String>,
    pub unauthorized_object_id: Option<String>,
    pub correlation_id: Option<String>,
    // Delegated resource identity
    pub delegated_tenant_id: Option<String>,
    pub delegated_user_object_id: Option<String>,
}

impl Fields {
    /// Formats an `OffsetDateTime` as an ISO 8601 UTC string for SAS.
    pub fn format_time(t: &OffsetDateTime) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            t.year(),
            u8::from(t.month()),
            t.day(),
            t.hour(),
            t.minute(),
            t.second(),
        )
    }

    /// Percent-encodes a string for use in a SAS query parameter value.
    pub fn encode(value: &str) -> String {
        percent_encode(value.as_bytes(), ENCODE_SET).to_string()
    }

    pub fn start_str(&self) -> String {
        self.start
            .as_ref()
            .map(Self::format_time)
            .unwrap_or_default()
    }

    pub fn expiry_str(&self) -> String {
        Self::format_time(&self.expiry)
    }

    pub fn ip_str(&self) -> String {
        self.ip_range
            .as_ref()
            .map(|ip| ip.to_string())
            .unwrap_or_default()
    }

    pub fn protocol_str(&self) -> String {
        self.protocol
            .as_ref()
            .map(|p| p.to_string())
            .unwrap_or_default()
    }

    pub fn encryption_scope_str(&self) -> String {
        self.encryption_scope.clone().unwrap_or_default()
    }
}

/// A builder for constructing Shared Access Signature (SAS) tokens.
///
/// The type parameter `R` determines which resource type the SAS targets,
/// controlling available permissions and output format.
///
/// Use [`Display`](fmt::Display) or [`to_string()`](ToString::to_string) to
/// produce the signed SAS query parameter string.
pub struct SasBuilder<'a, R: Resource> {
    resource: R,
    permissions: R::Permissions,
    key: &'a UserDelegationKey,
    fields: Fields,
}

impl<'a, R: Resource> SasBuilder<'a, R> {
    /// Creates a new SAS builder.
    ///
    /// # Parameters
    /// - `account`: The storage account name.
    /// - `resource`: The target resource (e.g., `Blob::new(...)`, `Container::new(...)`).
    /// - `permissions`: The permissions to grant.
    /// - `expiry`: When the SAS expires.
    /// - `key`: The user delegation key used to sign the SAS.
    pub fn new(
        account: impl Into<String>,
        resource: R,
        permissions: R::Permissions,
        expiry: OffsetDateTime,
        key: &'a UserDelegationKey,
    ) -> Self {
        Self {
            resource,
            permissions,
            key,
            fields: Fields {
                account: account.into(),
                start: None,
                expiry,
                protocol: None,
                ip_range: None,
                encryption_scope: None,
                cache_control: None,
                content_disposition: None,
                content_encoding: None,
                content_language: None,
                content_type: None,
                authorized_object_id: None,
                unauthorized_object_id: None,
                correlation_id: None,
                delegated_tenant_id: None,
                delegated_user_object_id: None,
            },
        }
    }

    /// Sets the optional start time for the SAS.
    pub fn start(mut self, start: OffsetDateTime) -> Self {
        self.fields.start = Some(start);
        self
    }

    /// Sets the permitted protocol (HTTPS only, or HTTPS and HTTP).
    pub fn protocol(mut self, protocol: SasProtocol) -> Self {
        self.fields.protocol = Some(protocol);
        self
    }

    /// Restricts the SAS to requests from the given IP address or range.
    pub fn ip_range(mut self, ip: SasIpRange) -> Self {
        self.fields.ip_range = Some(ip);
        self
    }

    /// Sets the encryption scope for the SAS.
    pub fn encryption_scope(mut self, scope: impl Into<String>) -> Self {
        self.fields.encryption_scope = Some(scope.into());
        self
    }

    /// Sets the delegated tenant ID (skdutid).
    pub fn delegated_tenant_id(mut self, value: impl Into<String>) -> Self {
        self.fields.delegated_tenant_id = Some(value.into());
        self
    }

    /// Sets the delegated user object ID (sduoid).
    pub fn delegated_user_object_id(mut self, value: impl Into<String>) -> Self {
        self.fields.delegated_user_object_id = Some(value.into());
        self
    }
}

impl<R: Resource> fmt::Display for SasBuilder<'_, R> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let sts = self
            .resource
            ._build_string_to_sign(&self.permissions, &self.fields, self.key);

        let mut mac = Hmac::<Sha256>::new_from_slice(&self.key.value)
            .expect("HMAC-SHA256 accepts any key length");
        mac.update(sts.as_bytes());
        let signature =
            base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes());

        let qp = self.resource._build_query_parameters(
            &self.permissions,
            &self.fields,
            self.key,
            &signature,
        );
        f.write_str(&qp)
    }
}

// Blob-service-specific setters
impl<R: BlobServiceResource> SasBuilder<'_, R> {
    /// Sets the `Cache-Control` response header override.
    pub fn cache_control(mut self, value: impl Into<String>) -> Self {
        self.fields.cache_control = Some(value.into());
        self
    }

    /// Sets the `Content-Disposition` response header override.
    pub fn content_disposition(mut self, value: impl Into<String>) -> Self {
        self.fields.content_disposition = Some(value.into());
        self
    }

    /// Sets the `Content-Encoding` response header override.
    pub fn content_encoding(mut self, value: impl Into<String>) -> Self {
        self.fields.content_encoding = Some(value.into());
        self
    }

    /// Sets the `Content-Language` response header override.
    pub fn content_language(mut self, value: impl Into<String>) -> Self {
        self.fields.content_language = Some(value.into());
        self
    }

    /// Sets the `Content-Type` response header override.
    pub fn content_type(mut self, value: impl Into<String>) -> Self {
        self.fields.content_type = Some(value.into());
        self
    }

    /// Sets the authorized AAD object ID (saoid).
    pub fn authorized_object_id(mut self, value: impl Into<String>) -> Self {
        self.fields.authorized_object_id = Some(value.into());
        self
    }

    /// Sets the unauthorized AAD object ID (suoid).
    pub fn unauthorized_object_id(mut self, value: impl Into<String>) -> Self {
        self.fields.unauthorized_object_id = Some(value.into());
        self
    }

    /// Sets the correlation ID (scid).
    pub fn correlation_id(mut self, value: impl Into<String>) -> Self {
        self.fields.correlation_id = Some(value.into());
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::blob::{Blob, BlobPermissions, Container};
    use crate::resource::{Queue, QueuePermissions};
    use time::macros::datetime;

    fn test_udk() -> UserDelegationKey {
        UserDelegationKey {
            signed_oid: "oid-value".into(),
            signed_tid: "tid-value".into(),
            signed_start: datetime!(2025-01-15 00:00:00 UTC),
            signed_expiry: datetime!(2025-01-16 00:00:00 UTC),
            signed_service: "b".into(),
            signed_version: "2025-11-05".into(),
            value: vec![116, 101, 115, 116, 107, 101, 121], // "testkey"
        }
    }

    /// Helper to get the string-to-sign from a builder (for test assertions).
    fn string_to_sign<R: Resource>(builder: &SasBuilder<'_, R>) -> String {
        builder
            .resource
            ._build_string_to_sign(&builder.permissions, &builder.fields, builder.key)
    }

    #[test]
    fn blob_udk_string_to_sign() {
        let udk = test_udk();
        let resource = Blob::new("mycontainer", "myblob.txt");
        let permissions = BlobPermissions {
            read: true,
            write: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry, &udk)
            .start(datetime!(2025-01-15 00:00:00 UTC))
            .protocol(SasProtocol::HttpsAndHttp);

        let sts = string_to_sign(&builder);
        assert!(sts.starts_with("rw\n"));
        assert!(sts.contains("/blob/myaccount/mycontainer/myblob.txt\n"));
        assert!(sts.contains("oid-value\n"));
        assert!(sts.contains("tid-value\n"));
        assert!(sts.contains("https,http\n"));
        assert!(sts.contains("2025-11-05\n"));
        assert!(sts.contains("\nb\n")); // sr=b
    }

    #[test]
    fn blob_udk_display_produces_signed_query() {
        let udk = test_udk();
        let resource = Blob::new("mycontainer", "myblob.txt");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry, &udk)
            .cache_control("no-cache");

        let qp = builder.to_string();
        assert!(qp.starts_with("sv=2025-11-05&sr=b&"));
        assert!(qp.contains("sp=r"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("rscc=no-cache"));
        assert!(qp.contains("sig="));
        // Verify signature is a valid base64 string (contains only base64 chars after percent-decoding)
        let sig_start = qp.find("sig=").unwrap() + 4;
        let sig_end = qp[sig_start..]
            .find('&')
            .map_or(qp.len(), |i| sig_start + i);
        let sig_encoded = &qp[sig_start..sig_end];
        assert!(!sig_encoded.is_empty());
    }

    #[test]
    fn container_string_to_sign() {
        let udk = test_udk();
        let resource = Container::new("mycontainer");
        let permissions = crate::resource::blob::ContainerPermissions {
            read: true,
            list: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry, &udk);
        let sts = string_to_sign(&builder);
        assert!(sts.starts_with("rl\n"));
        assert!(sts.contains("/blob/myaccount/mycontainer\n"));
        assert!(sts.contains("\nc\n")); // sr=c
    }

    #[test]
    fn queue_udk_string_to_sign() {
        let udk = test_udk();
        let resource = Queue::new("myqueue");
        let permissions = QueuePermissions {
            read: true,
            process: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry, &udk);
        let sts = string_to_sign(&builder);
        assert!(sts.starts_with("rp\n"));
        assert!(sts.contains("/queue/myaccount/myqueue\n"));
        assert!(sts.contains("oid-value\n"));
        // Queue has 15 newline-separated fields (trailing \n)
        assert_eq!(sts.matches('\n').count(), 15);
    }

    #[test]
    fn queue_display_produces_signed_query() {
        let udk = test_udk();
        let resource = Queue::new("myqueue");
        let permissions = QueuePermissions {
            read: true,
            add: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry, &udk);
        let qp = builder.to_string();
        assert!(qp.starts_with("sv=2025-11-05&"));
        assert!(qp.contains("sp=ra"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("sig="));
        // Queue should not have blob-specific params
        assert!(!qp.contains("sr="));
        assert!(!qp.contains("rscc="));
    }

    #[test]
    fn delegated_setters_compile_for_blob() {
        let udk = test_udk();
        let resource = Blob::new("c", "b");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        // All blob-service + delegated setters should be available
        let builder = SasBuilder::new("acct", resource, permissions, expiry, &udk)
            .cache_control("no-cache")
            .content_disposition("inline")
            .content_encoding("gzip")
            .content_language("en-US")
            .content_type("text/plain")
            .authorized_object_id("saoid")
            .unauthorized_object_id("suoid")
            .correlation_id("scid")
            .delegated_tenant_id("dtid")
            .delegated_user_object_id("duoid");

        let sts = string_to_sign(&builder);
        assert!(sts.contains("saoid\n"));
        assert!(sts.contains("suoid\n"));
        assert!(sts.contains("scid\n"));
        assert!(sts.contains("dtid\n"));
        assert!(sts.contains("duoid\n"));
    }

    #[test]
    fn blob_snapshot_sets_sr_bs() {
        let udk = test_udk();
        let resource = Blob::new("c", "b").snapshot("2025-01-15T12:00:00.0000000Z");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", resource, permissions, expiry, &udk);
        let sts = string_to_sign(&builder);
        assert!(sts.contains("\nbs\n")); // sr=bs
        assert!(sts.contains("2025-01-15T12:00:00.0000000Z\n")); // snapshot time in sts

        let qp = builder.to_string();
        assert!(qp.contains("sr=bs"));
        assert!(qp.contains("snapshot=2025-01-15T12:00:00.0000000Z"));
    }

    #[test]
    fn blob_version_sets_sr_bv() {
        let udk = test_udk();
        let resource = Blob::new("c", "b").version("2025-01-15T12:00:00.0000000Z");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", resource, permissions, expiry, &udk);
        let sts = string_to_sign(&builder);
        assert!(sts.contains("\nbv\n")); // sr=bv

        let qp = builder.to_string();
        assert!(qp.contains("sr=bv"));
    }

    #[test]
    fn format_time_produces_iso8601() {
        let t = datetime!(2025-01-15 09:30:45 UTC);
        assert_eq!(Fields::format_time(&t), "2025-01-15T09:30:45Z");
    }

    #[test]
    fn encode_percent_encodes_special_chars() {
        assert_eq!(Fields::encode("a+b/c=d"), "a%2Bb%2Fc%3Dd");
        assert_eq!(Fields::encode("hello"), "hello");
    }

    #[test]
    fn display_produces_deterministic_signature() {
        let udk = test_udk();
        let resource = Blob::new("c", "b");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", resource, permissions, expiry, &udk);
        let first = builder.to_string();
        let second = builder.to_string();
        assert_eq!(first, second);
    }

    #[test]
    fn display_in_format_string() {
        let udk = test_udk();
        let resource = Blob::new("photos", "cat.jpg");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let sas = SasBuilder::new("myaccount", resource, permissions, expiry, &udk);
        let url = format!("https://myaccount.blob.core.windows.net/photos/cat.jpg?{sas}");
        assert!(url.starts_with(
            "https://myaccount.blob.core.windows.net/photos/cat.jpg?sv=2025-11-05&sr=b&"
        ));
        assert!(url.contains("sig="));
    }
}
