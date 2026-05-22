// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Type-safe Shared Access Signature (SAS) builder for Azure Storage.
//!
//! This crate helps construct valid SAS query parameters for Azure Storage
//! resources. It does **not** perform HMAC signing — callers provide the
//! signature externally after computing it over the string-to-sign.
//!
//! # Supported resource types
//!
//! - [`Account`](resource::Account) — account-level SAS
//! - [`Blob`](resource::blob::Blob) — blob-level user delegation SAS (also covers snapshots and versions)
//! - [`Container`](resource::blob::Container) — container-level user delegation SAS
//! - [`Directory`](resource::blob::Directory) — directory-level (ADLS Gen2) user delegation SAS
//! - [`Queue`](resource::Queue) — queue-level user delegation SAS
//!
//! # Examples
//!
//! ## Account SAS (read + list blobs)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, SasProtocol, resource::{Account, AccountServices, AccountResourceTypes, AccountPermissions}};
//! use time::OffsetDateTime;
//!
//! let resource = Account::new(
//!     AccountServices { blob: true, ..Default::default() },
//!     AccountResourceTypes { container: true, object: true, ..Default::default() },
//! );
//! let permissions = AccountPermissions { read: true, list: true, ..Default::default() };
//! let expiry = OffsetDateTime::now_utc() + time::Duration::hours(1);
//!
//! let builder = SasBuilder::new("myaccount", resource, permissions, expiry)
//!     .protocol(SasProtocol::Https);
//!
//! let sts = builder.string_to_sign(&());
//! // sign: base64(hmac_sha256(base64_decode(account_key), sts.as_bytes()))
//! let signature = "computed-signature";
//! let query = builder.query_parameters(&(), signature);
//! // => "sv=2025-11-05&ss=b&srt=co&sp=rl&se=...&spr=https&sig=..."
//! ```
//!
//! ## Blob user delegation SAS (read a specific blob)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Blob, BlobPermissions}};
//! use time::OffsetDateTime;
//!
//! let udk = UserDelegationKey {
//!     signed_oid: "object-id".into(),
//!     signed_tid: "tenant-id".into(),
//!     signed_start: "2025-05-21T00:00:00Z".into(),
//!     signed_expiry: "2025-05-22T00:00:00Z".into(),
//!     signed_service: "b".into(),
//!     signed_version: "2025-11-05".into(),
//!     value: "base64-key-value".into(),
//! };
//!
//! let resource = Blob::new("images", "photo.jpg");
//! let permissions = BlobPermissions { read: true, ..Default::default() };
//! let expiry = OffsetDateTime::now_utc() + time::Duration::hours(1);
//!
//! let builder = SasBuilder::new("myaccount", resource, permissions, expiry)
//!     .content_type("image/jpeg");
//!
//! let sts = builder.string_to_sign(&udk);
//! let signature = "computed-signature";
//! let query = builder.query_parameters(&udk, signature);
//! // => "sv=2025-11-05&sr=b&se=...&sp=r&skoid=...&rsct=image/jpeg&sig=..."
//! ```
//!
//! ## Blob snapshot SAS
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Blob, BlobPermissions}};
//! use time::OffsetDateTime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: "2025-05-21T00:00:00Z".into(),
//! #     signed_expiry: "2025-05-22T00:00:00Z".into(),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: "key".into(),
//! # };
//! let resource = Blob::new("backups", "db.bak")
//!     .snapshot("2025-05-20T10:00:00.0000000Z");
//!
//! let permissions = BlobPermissions { read: true, ..Default::default() };
//! let builder = SasBuilder::new("myaccount", resource, permissions,
//!     OffsetDateTime::now_utc() + time::Duration::hours(1));
//!
//! let sts = builder.string_to_sign(&udk);
//! // sr=bs in the output, snapshot time included in string-to-sign
//! ```
//!
//! ## Container SAS (list + read all blobs)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, SasIpRange, UserDelegationKey, resource::blob::{Container, ContainerPermissions}};
//! use std::net::Ipv4Addr;
//! use time::OffsetDateTime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: "2025-05-21T00:00:00Z".into(),
//! #     signed_expiry: "2025-05-22T00:00:00Z".into(),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: "key".into(),
//! # };
//! let resource = Container::new("logs");
//! let permissions = ContainerPermissions { read: true, list: true, ..Default::default() };
//! let expiry = OffsetDateTime::now_utc() + time::Duration::hours(4);
//!
//! let builder = SasBuilder::new("myaccount", resource, permissions, expiry)
//!     .ip_range(SasIpRange::Range {
//!         start: Ipv4Addr::new(10, 0, 0, 1).into(),
//!         end: Ipv4Addr::new(10, 0, 0, 255).into(),
//!     });
//!
//! let sts = builder.string_to_sign(&udk);
//! let query = builder.query_parameters(&udk, "sig");
//! // => "sv=2025-11-05&sr=c&...&sip=10.0.0.1-10.0.0.255&sp=rl&..."
//! ```
//!
//! ## Directory SAS (ADLS Gen2)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Directory, ContainerPermissions}};
//! use time::OffsetDateTime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: "2025-05-21T00:00:00Z".into(),
//! #     signed_expiry: "2025-05-22T00:00:00Z".into(),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: "key".into(),
//! # };
//! // Depth is computed automatically from the path (2 segments here)
//! let resource = Directory::new("filesystem", "path/to");
//! let permissions = ContainerPermissions { read: true, list: true, ..Default::default() };
//!
//! let builder = SasBuilder::new("myaccount", resource, permissions,
//!     OffsetDateTime::now_utc() + time::Duration::hours(1));
//!
//! let sts = builder.string_to_sign(&udk);
//! let query = builder.query_parameters(&udk, "sig");
//! // => "sv=2025-11-05&sr=d&...&sdd=2&..."
//! ```
//!
//! ## Queue SAS (read + process messages)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, SasProtocol, UserDelegationKey, resource::{Queue, QueuePermissions}};
//! use time::OffsetDateTime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: "2025-05-21T00:00:00Z".into(),
//! #     signed_expiry: "2025-05-22T00:00:00Z".into(),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: "key".into(),
//! # };
//! let resource = Queue::new("work-items");
//! let permissions = QueuePermissions { read: true, process: true, ..Default::default() };
//!
//! let builder = SasBuilder::new("myaccount", resource, permissions,
//!     OffsetDateTime::now_utc() + time::Duration::hours(8))
//!     .protocol(SasProtocol::Https)
//!     .delegated_tenant_id("tenant-id");
//!
//! let sts = builder.string_to_sign(&udk);
//! let query = builder.query_parameters(&udk, "sig");
//! // => "sv=2025-11-05&se=...&sp=rp&spr=https&skoid=...&skdutid=tenant-id&sig=..."
//! ```

mod builder;
mod ip_range;
mod key;
mod protocol;
pub mod resource;

pub use builder::SasBuilder;
pub use ip_range::SasIpRange;
pub use key::UserDelegationKey;
pub use protocol::SasProtocol;

/// The SAS service version targeted by this crate.
pub const SAS_VERSION: &str = "2025-11-05";
