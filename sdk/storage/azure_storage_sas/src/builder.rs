// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::ip_range::SasIpRange;
use crate::protocol::SasProtocol;
use crate::resource::blob::{
    blob_udk_query_parameters, blob_udk_string_to_sign, Blob, BlobPermissions, Container,
    ContainerPermissions, Directory,
};
use crate::resource::{Queue, QueuePermissions};
use crate::UserDelegationKey;
use crate::SAS_VERSION;
use base64::Engine;
use hmac::{Hmac, Mac};
use percent_encoding::{percent_encode, AsciiSet, NON_ALPHANUMERIC};
use sha2::Sha256;
use time::OffsetDateTime;

/// Characters that must be percent-encoded in SAS query parameter values.
const ENCODE_SET: &AsciiSet = &NON_ALPHANUMERIC
    .remove(b'-')
    .remove(b'_')
    .remove(b'.')
    .remove(b'~');

/// Typestate markers for [`SasBuilder`].
pub mod state {
    use crate::resource::blob::{
        Blob, BlobPermissions, Container, ContainerPermissions, Directory,
    };
    use crate::resource::{Queue, QueuePermissions};

    /// Initial state before a resource type has been selected.
    pub struct Untyped;

    /// State after selecting a blob resource.
    pub struct BlobState {
        pub(crate) resource: Blob,
        pub(crate) permissions: BlobPermissions,
    }

    /// State after selecting a container resource.
    pub struct ContainerState {
        pub(crate) resource: Container,
        pub(crate) permissions: ContainerPermissions,
    }

    /// State after selecting a directory resource.
    pub struct DirectoryState {
        pub(crate) resource: Directory,
        pub(crate) permissions: ContainerPermissions,
    }

    /// State after selecting a queue resource.
    pub struct QueueState {
        pub(crate) resource: Queue,
        pub(crate) permissions: QueuePermissions,
    }
}

mod sealed {
    pub trait Sealed {}
    impl Sealed for super::state::BlobState {}
    impl Sealed for super::state::ContainerState {}
    impl Sealed for super::state::DirectoryState {}
}

/// Marker trait for blob-service typestate markers.
///
/// Types implementing this trait support response header overrides and
/// other blob-service-specific SAS fields.
pub trait BlobServiceState: sealed::Sealed {}
impl BlobServiceState for state::BlobState {}
impl BlobServiceState for state::ContainerState {}
impl BlobServiceState for state::DirectoryState {}

/// Internal fields shared across all builder states.
pub(crate) struct Fields {
    pub account: String,
    pub start: Option<OffsetDateTime>,
    pub expiry: OffsetDateTime,
    pub protocol: Option<SasProtocol>,
    pub ip_range: Option<SasIpRange>,
    pub encryption_scope: Option<String>,
    pub cache_control: Option<String>,
    pub content_disposition: Option<String>,
    pub content_encoding: Option<String>,
    pub content_language: Option<String>,
    pub content_type: Option<String>,
    pub authorized_object_id: Option<String>,
    pub unauthorized_object_id: Option<String>,
    pub correlation_id: Option<String>,
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
/// The type parameter `S` tracks the builder state, gating which methods
/// are available at compile time. Call a resource method (e.g., [`.blob()`](SasBuilder::blob))
/// to transition from [`Untyped`](state::Untyped) to a typed state, then call
/// [`.build()`](SasBuilder::build) to produce the signed query string.
pub struct SasBuilder<'a, S> {
    key: &'a UserDelegationKey,
    fields: Fields,
    state: S,
}

impl<'a> SasBuilder<'a, state::Untyped> {
    /// Creates a new SAS builder.
    ///
    /// # Parameters
    /// - `account`: The storage account name.
    /// - `key`: The user delegation key used to sign the SAS.
    /// - `expiry`: When the SAS expires.
    pub fn new(
        account: impl Into<String>,
        key: &'a UserDelegationKey,
        expiry: OffsetDateTime,
    ) -> Self {
        Self {
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
            state: state::Untyped,
        }
    }

    /// Selects a blob resource and transitions the builder to blob state.
    pub fn blob(
        self,
        resource: Blob,
        permissions: BlobPermissions,
    ) -> SasBuilder<'a, state::BlobState> {
        SasBuilder {
            key: self.key,
            fields: self.fields,
            state: state::BlobState {
                resource,
                permissions,
            },
        }
    }

    /// Selects a container resource and transitions the builder to container state.
    pub fn container(
        self,
        resource: Container,
        permissions: ContainerPermissions,
    ) -> SasBuilder<'a, state::ContainerState> {
        SasBuilder {
            key: self.key,
            fields: self.fields,
            state: state::ContainerState {
                resource,
                permissions,
            },
        }
    }

    /// Selects a directory resource and transitions the builder to directory state.
    pub fn directory(
        self,
        resource: Directory,
        permissions: ContainerPermissions,
    ) -> SasBuilder<'a, state::DirectoryState> {
        SasBuilder {
            key: self.key,
            fields: self.fields,
            state: state::DirectoryState {
                resource,
                permissions,
            },
        }
    }

    /// Selects a queue resource and transitions the builder to queue state.
    pub fn queue(
        self,
        resource: Queue,
        permissions: QueuePermissions,
    ) -> SasBuilder<'a, state::QueueState> {
        SasBuilder {
            key: self.key,
            fields: self.fields,
            state: state::QueueState {
                resource,
                permissions,
            },
        }
    }
}

// Common setters available in any state.
impl<S> SasBuilder<'_, S> {
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

// Blob-service-specific setters.
impl<S: BlobServiceState> SasBuilder<'_, S> {
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

impl SasBuilder<'_, state::BlobState> {
    /// Builds the signed SAS query parameter string.
    pub fn build(&self) -> String {
        let sts = blob_udk_string_to_sign(
            &self.state.permissions,
            &self.fields,
            self.key,
            self.state.resource.signed_resource(),
            &self
                .state
                .resource
                .canonicalized_resource(&self.fields.account),
            self.state.resource.snapshot_time().unwrap_or(""),
            "",
        );
        let signature = sign(&self.key.value, &sts);
        blob_udk_query_parameters(
            &self.state.permissions,
            &self.fields,
            self.key,
            self.state.resource.signed_resource(),
            self.state.resource.snapshot_time(),
            None,
            &signature,
        )
    }
}

impl SasBuilder<'_, state::ContainerState> {
    /// Builds the signed SAS query parameter string.
    pub fn build(&self) -> String {
        let sts = blob_udk_string_to_sign(
            &self.state.permissions,
            &self.fields,
            self.key,
            "c",
            &self
                .state
                .resource
                .canonicalized_resource(&self.fields.account),
            "",
            "",
        );
        let signature = sign(&self.key.value, &sts);
        blob_udk_query_parameters(
            &self.state.permissions,
            &self.fields,
            self.key,
            "c",
            None,
            None,
            &signature,
        )
    }
}

impl SasBuilder<'_, state::DirectoryState> {
    /// Builds the signed SAS query parameter string.
    pub fn build(&self) -> String {
        let depth = self.state.resource.depth();
        let depth_str = depth.to_string();
        let sts = blob_udk_string_to_sign(
            &self.state.permissions,
            &self.fields,
            self.key,
            "d",
            &self
                .state
                .resource
                .canonicalized_resource(&self.fields.account),
            "",
            &depth_str,
        );
        let signature = sign(&self.key.value, &sts);
        blob_udk_query_parameters(
            &self.state.permissions,
            &self.fields,
            self.key,
            "d",
            None,
            Some(depth),
            &signature,
        )
    }
}

impl SasBuilder<'_, state::QueueState> {
    /// Builds the signed SAS query parameter string.
    pub fn build(&self) -> String {
        let sts = format!(
            "{sp}\n{st}\n{se}\n{cr}\n\
             {skoid}\n{sktid}\n{skt}\n{ske}\n{sks}\n{skv}\n\
             {skdutid}\n{sduoid}\n\
             {sip}\n{spr}\n{sv}\n",
            sp = self.state.permissions,
            st = self.fields.start_str(),
            se = self.fields.expiry_str(),
            cr = self
                .state
                .resource
                .canonicalized_resource(&self.fields.account),
            skoid = self.key.signed_oid,
            sktid = self.key.signed_tid,
            skt = Fields::format_time(&self.key.signed_start),
            ske = Fields::format_time(&self.key.signed_expiry),
            sks = self.key.signed_service,
            skv = self.key.signed_version,
            skdutid = self.fields.delegated_tenant_id.as_deref().unwrap_or(""),
            sduoid = self
                .fields
                .delegated_user_object_id
                .as_deref()
                .unwrap_or(""),
            sip = self.fields.ip_str(),
            spr = self.fields.protocol_str(),
            sv = SAS_VERSION,
        );
        let signature = sign(&self.key.value, &sts);

        let mut parts = Vec::with_capacity(15);
        parts.push(format!("sv={SAS_VERSION}"));
        if let Some(ref start) = self.fields.start {
            parts.push(format!("st={}", Fields::format_time(start)));
        }
        parts.push(format!("se={}", self.fields.expiry_str()));
        parts.push(format!("sp={}", self.state.permissions));
        if let Some(ref ip) = self.fields.ip_range {
            parts.push(format!("sip={ip}"));
        }
        if let Some(ref proto) = self.fields.protocol {
            parts.push(format!("spr={proto}"));
        }
        parts.push(format!("skoid={}", self.key.signed_oid));
        parts.push(format!("sktid={}", self.key.signed_tid));
        parts.push(format!(
            "skt={}",
            Fields::format_time(&self.key.signed_start)
        ));
        parts.push(format!(
            "ske={}",
            Fields::format_time(&self.key.signed_expiry)
        ));
        parts.push(format!("sks={}", self.key.signed_service));
        parts.push(format!("skv={}", self.key.signed_version));
        if let Some(ref v) = self.fields.delegated_tenant_id {
            parts.push(format!("skdutid={v}"));
        }
        if let Some(ref v) = self.fields.delegated_user_object_id {
            parts.push(format!("sduoid={v}"));
        }
        parts.push(format!("sig={}", Fields::encode(&signature)));
        parts.join("&")
    }
}

/// Computes an HMAC-SHA256 signature and returns it as a base64 string.
fn sign(key: &[u8], message: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC-SHA256 accepts any key length");
    mac.update(message.as_bytes());
    base64::engine::general_purpose::STANDARD.encode(mac.finalize().into_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resource::blob::{Blob, BlobPermissions, Container, ContainerPermissions};
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

    #[test]
    fn blob_string_to_sign() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", &udk, expiry)
            .start(datetime!(2025-01-15 00:00:00 UTC))
            .protocol(SasProtocol::HttpsAndHttp)
            .blob(
                Blob::new("mycontainer", "myblob.txt"),
                BlobPermissions::new().read().write(),
            );

        let qp = builder.build();
        assert!(qp.contains("sp=rw"));
        assert!(qp.contains("sr=b"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("spr=https%2Chttp") || qp.contains("spr=https,http"));
        assert!(qp.contains("sig="));
    }

    #[test]
    fn blob_build_produces_signed_query() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", &udk, expiry)
            .blob(
                Blob::new("mycontainer", "myblob.txt"),
                BlobPermissions::new().read(),
            )
            .cache_control("no-cache");

        let qp = builder.build();
        assert!(qp.starts_with("sv=2025-11-05&sr=b&"));
        assert!(qp.contains("sp=r"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("rscc=no-cache"));
        assert!(qp.contains("sig="));
    }

    #[test]
    fn container_build() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", &udk, expiry).container(
            Container::new("mycontainer"),
            ContainerPermissions::new().read().list(),
        );

        let qp = builder.build();
        assert!(qp.starts_with("sv=2025-11-05&sr=c&"));
        assert!(qp.contains("sp=rl"));
        assert!(qp.contains("sig="));
    }

    #[test]
    fn queue_build() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("myaccount", &udk, expiry)
            .queue(Queue::new("myqueue"), QueuePermissions::new().read().add());

        let qp = builder.build();
        assert!(qp.starts_with("sv=2025-11-05&"));
        assert!(qp.contains("sp=ra"));
        assert!(qp.contains("skoid=oid-value"));
        assert!(qp.contains("sig="));
        // Queue should not have blob-specific params
        assert!(!qp.contains("sr="));
        assert!(!qp.contains("rscc="));
    }

    #[test]
    fn delegated_setters_on_blob() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", &udk, expiry)
            .delegated_tenant_id("dtid")
            .delegated_user_object_id("duoid")
            .blob(Blob::new("c", "b"), BlobPermissions::new().read())
            .cache_control("no-cache")
            .content_disposition("inline")
            .content_encoding("gzip")
            .content_language("en-US")
            .content_type("text/plain")
            .authorized_object_id("saoid")
            .unauthorized_object_id("suoid")
            .correlation_id("scid");

        let qp = builder.build();
        assert!(qp.contains("skdutid=dtid"));
        assert!(qp.contains("sduoid=duoid"));
        assert!(qp.contains("saoid=saoid"));
        assert!(qp.contains("suoid=suoid"));
        assert!(qp.contains("scid=scid"));
        assert!(qp.contains("rscc=no-cache"));
        assert!(qp.contains("rsct=text%2Fplain") || qp.contains("rsct=text/plain"));
    }

    #[test]
    fn blob_snapshot_sets_sr_bs() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", &udk, expiry).blob(
            Blob::new("c", "b").snapshot("2025-01-15T12:00:00.0000000Z"),
            BlobPermissions::new().read(),
        );

        let qp = builder.build();
        assert!(qp.contains("sr=bs"));
        assert!(qp.contains("snapshot=2025-01-15T12:00:00.0000000Z"));
    }

    #[test]
    fn blob_version_sets_sr_bv() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", &udk, expiry).blob(
            Blob::new("c", "b").version("2025-01-15T12:00:00.0000000Z"),
            BlobPermissions::new().read(),
        );

        let qp = builder.build();
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
    fn build_produces_deterministic_signature() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        let builder = SasBuilder::new("acct", &udk, expiry)
            .blob(Blob::new("c", "b"), BlobPermissions::new().read());

        let first = builder.build();
        let second = builder.build();
        assert_eq!(first, second);
    }

    #[test]
    fn common_setters_before_and_after_transition() {
        let udk = test_udk();
        let expiry = datetime!(2025-06-01 12:00:00 UTC);

        // Common setters work before transition
        let builder = SasBuilder::new("acct", &udk, expiry)
            .protocol(SasProtocol::Https)
            .blob(Blob::new("c", "b"), BlobPermissions::new().read())
            .start(datetime!(2025-01-15 00:00:00 UTC));

        let qp = builder.build();
        assert!(qp.contains("spr=https"));
        assert!(qp.contains("st=2025-01-15T00:00:00Z"));
    }
}
