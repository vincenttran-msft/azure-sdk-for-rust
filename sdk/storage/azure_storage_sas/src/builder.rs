// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::ip_range::SasIpRange;
use crate::protocol::SasProtocol;
use crate::resource::{BlobServiceResource, DelegatedResource, Resource};
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
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
/// controlling available permissions, required signing context, and output format.
pub struct SasBuilder<R: Resource> {
    resource: R,
    permissions: R::Permissions,
    fields: Fields,
}

impl<R: Resource> SasBuilder<R> {
    /// Creates a new SAS builder.
    ///
    /// # Parameters
    /// - `account`: The storage account name.
    /// - `resource`: The target resource (e.g., `Blob::new(...)`, `Account::new(...)`).
    /// - `permissions`: The permissions to grant.
    /// - `expiry`: When the SAS expires.
    pub fn new(
        account: impl Into<String>,
        resource: R,
        permissions: R::Permissions,
        expiry: OffsetDateTime,
    ) -> Self {
        Self {
            resource,
            permissions,
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

    /// Builds the string-to-sign for this SAS.
    ///
    /// The caller should compute `HMAC-SHA256(key, string_to_sign)` and
    /// base64-encode the result to produce the signature.
    ///
    /// For account SAS, `context` is `&()`. For user delegation SAS,
    /// `context` is a `&UserDelegationKey`.
    pub fn string_to_sign(&self, context: &R::SigningContext) -> String {
        self.resource
            ._build_string_to_sign(&self.permissions, &self.fields, context)
    }

    /// Builds the SAS query parameter string (without leading `?`).
    ///
    /// `signature` is the base64-encoded HMAC-SHA256 of the string-to-sign.
    pub fn query_parameters(&self, context: &R::SigningContext, signature: &str) -> String {
        self.resource
            ._build_query_parameters(&self.permissions, &self.fields, context, signature)
    }
}

// Blob-service-specific setters
impl<R: BlobServiceResource> SasBuilder<R> {
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

// Delegated resource setters (blob-service + queue)
impl<R: DelegatedResource> SasBuilder<R> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::blob::{Blob, BlobPermissions, Container};
    use crate::resource::{
        Account, AccountPermissions, AccountResourceTypes, AccountServices, Queue, QueuePermissions,
    };
    use crate::UserDelegationKey;
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

    #[test]
    fn account_sas_string_to_sign() {
        let resource = Account::new(
            AccountServices {
                blob: true,
                queue: true,
                ..Default::default()
            },
            AccountResourceTypes {
                service: true,
                container: true,
                object: true,
            },
        );
        let permissions = AccountPermissions {
            read: true,
            write: true,
            list: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let builder = SasBuilder::new("myaccount", resource, permissions, expiry)
            .start(datetime!(2025-01-15 00:00:00 UTC))
            .protocol(SasProtocol::Https);

        let sts = builder.string_to_sign(&());
        let expected = "myaccount\nrwl\nbq\nsco\n2025-01-15T00:00:00Z\n2025-06-01T12:00:00Z\n\nhttps\n2025-11-05\n\n";
        assert_eq!(sts, expected);
    }

    #[test]
    fn account_sas_query_parameters() {
        let resource = Account::new(
            AccountServices {
                blob: true,
                ..Default::default()
            },
            AccountResourceTypes {
                object: true,
                ..Default::default()
            },
        );
        let permissions = AccountPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let builder = SasBuilder::new("myaccount", resource, permissions, expiry);

        let qp = builder.query_parameters(&(), "fakesig+/=");
        assert!(qp.starts_with("sv=2025-11-05&"));
        assert!(qp.contains("ss=b"));
        assert!(qp.contains("srt=o"));
        assert!(qp.contains("sp=r"));
        assert!(qp.contains("se=2025-06-01T12:00:00Z"));
        assert!(qp.contains("sig=fakesig%2B%2F%3D"));
    }

    #[test]
    fn blob_udk_string_to_sign() {
        let resource = Blob::new("mycontainer", "myblob.txt");
        let permissions = BlobPermissions {
            read: true,
            write: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry)
            .start(datetime!(2025-01-15 00:00:00 UTC))
            .protocol(SasProtocol::HttpsAndHttp);

        let sts = builder.string_to_sign(&udk);
        // Verify key fields are present
        assert!(sts.starts_with("rw\n"));
        assert!(sts.contains("/blob/myaccount/mycontainer/myblob.txt\n"));
        assert!(sts.contains("oid-value\n"));
        assert!(sts.contains("tid-value\n"));
        assert!(sts.contains("https,http\n"));
        assert!(sts.contains("2025-11-05\n"));
        assert!(sts.contains("\nb\n")); // sr=b
    }

    #[test]
    fn blob_udk_query_parameters() {
        let resource = Blob::new("mycontainer", "myblob.txt");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder =
            SasBuilder::new("myaccount", resource, permissions, expiry).cache_control("no-cache");

        let qp = builder.query_parameters(&udk, "mysig");
        assert!(qp.starts_with("sv=2025-11-05&sr=b&"));
        assert!(qp.contains("sp=r"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("rscc=no-cache"));
        assert!(qp.contains("sig=mysig"));
    }

    #[test]
    fn container_string_to_sign() {
        let resource = Container::new("mycontainer");
        let permissions = crate::resource::blob::ContainerPermissions {
            read: true,
            list: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry);
        let sts = builder.string_to_sign(&udk);
        assert!(sts.starts_with("rl\n"));
        assert!(sts.contains("/blob/myaccount/mycontainer\n"));
        assert!(sts.contains("\nc\n")); // sr=c
    }

    #[test]
    fn queue_udk_string_to_sign() {
        let resource = Queue::new("myqueue");
        let permissions = QueuePermissions {
            read: true,
            process: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry);
        let sts = builder.string_to_sign(&udk);
        assert!(sts.starts_with("rp\n"));
        assert!(sts.contains("/queue/myaccount/myqueue\n"));
        assert!(sts.contains("oid-value\n"));
        // Queue has 15 newline-separated fields (trailing \n)
        assert_eq!(sts.matches('\n').count(), 15);
    }

    #[test]
    fn queue_query_parameters() {
        let resource = Queue::new("myqueue");
        let permissions = QueuePermissions {
            read: true,
            add: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("myaccount", resource, permissions, expiry);
        let qp = builder.query_parameters(&udk, "qsig");
        assert!(qp.starts_with("sv=2025-11-05&"));
        assert!(qp.contains("sp=ra"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("sig=qsig"));
        // Queue should not have blob-specific params
        assert!(!qp.contains("sr="));
        assert!(!qp.contains("rscc="));
    }

    #[test]
    fn delegated_setters_compile_for_blob() {
        let resource = Blob::new("c", "b");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        // All blob-service + delegated setters should be available
        let builder = SasBuilder::new("acct", resource, permissions, expiry)
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

        let udk = test_udk();
        let sts = builder.string_to_sign(&udk);
        assert!(sts.contains("saoid\n"));
        assert!(sts.contains("suoid\n"));
        assert!(sts.contains("scid\n"));
        assert!(sts.contains("dtid\n"));
        assert!(sts.contains("duoid\n"));
    }

    #[test]
    fn blob_snapshot_sets_sr_bs() {
        let resource = Blob::new("c", "b").snapshot("2025-01-15T12:00:00.0000000Z");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("acct", resource, permissions, expiry);
        let sts = builder.string_to_sign(&udk);
        assert!(sts.contains("\nbs\n")); // sr=bs
        assert!(sts.contains("2025-01-15T12:00:00.0000000Z\n")); // snapshot time in sts

        let qp = builder.query_parameters(&udk, "sig");
        assert!(qp.contains("sr=bs"));
        assert!(qp.contains("snapshot=2025-01-15T12:00:00.0000000Z"));
    }

    #[test]
    fn blob_version_sets_sr_bv() {
        let resource = Blob::new("c", "b").version("2025-01-15T12:00:00.0000000Z");
        let permissions = BlobPermissions {
            read: true,
            ..Default::default()
        };
        let expiry = datetime!(2025-06-01 12:00:00 UTC);
        let udk = test_udk();

        let builder = SasBuilder::new("acct", resource, permissions, expiry);
        let sts = builder.string_to_sign(&udk);
        assert!(sts.contains("\nbv\n")); // sr=bv

        let qp = builder.query_parameters(&udk, "sig");
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
}
