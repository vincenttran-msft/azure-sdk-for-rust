// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Type-safe Shared Access Signature (SAS) builder for Azure Storage.
//!
//! This crate constructs signed SAS query parameter strings for Azure Storage
//! resources using a user delegation key. The signing (HMAC-SHA256) is handled
//! internally — just use [`Display`](std::fmt::Display) or
//! [`to_string()`](ToString::to_string) on the builder to get the final query string.
//!
//! # Supported resource types
//!
//! - [`Blob`](resource::blob::Blob) — blob-level user delegation SAS (also covers snapshots and versions)
//! - [`Container`](resource::blob::Container) — container-level user delegation SAS
//! - [`Directory`](resource::blob::Directory) — directory-level (ADLS Gen2) user delegation SAS
//! - [`Queue`](resource::Queue) — queue-level user delegation SAS
//!
//! # Examples
//!
//! ## Blob user delegation SAS (read a specific blob)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Blob, BlobPermissions}};
//! use time::OffsetDateTime;
//! use time::macros::datetime;
//!
//! let udk = UserDelegationKey {
//!     signed_oid: "object-id".into(),
//!     signed_tid: "tenant-id".into(),
//!     signed_start: datetime!(2025-05-21 00:00:00 UTC),
//!     signed_expiry: datetime!(2025-05-22 00:00:00 UTC),
//!     signed_service: "b".into(),
//!     signed_version: "2025-11-05".into(),
//!     value: vec![0; 32], // decoded key bytes
//! };
//!
//! let sas = SasBuilder::new("myaccount", Blob::new("images", "photo.jpg"),
//!     BlobPermissions { read: true, ..Default::default() },
//!     OffsetDateTime::now_utc() + time::Duration::hours(1), &udk)
//!     .content_type("image/jpeg");
//!
//! let url = format!("https://myaccount.blob.core.windows.net/images/photo.jpg?{sas}");
//! // => "https://myaccount.blob.core.windows.net/images/photo.jpg?sv=2025-11-05&sr=b&..."
//! ```
//!
//! ## Blob snapshot SAS
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Blob, BlobPermissions}};
//! use time::OffsetDateTime;
//! use time::macros::datetime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: datetime!(2025-05-21 00:00:00 UTC),
//! #     signed_expiry: datetime!(2025-05-22 00:00:00 UTC),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: vec![0; 32],
//! # };
//! let sas = SasBuilder::new("myaccount",
//!     Blob::new("backups", "db.bak").snapshot("2025-05-20T10:00:00.0000000Z"),
//!     BlobPermissions { read: true, ..Default::default() },
//!     OffsetDateTime::now_utc() + time::Duration::hours(1), &udk);
//!
//! // sr=bs in the output, snapshot time included in the signed token
//! let query = sas.to_string();
//! ```
//!
//! ## Container SAS (list + read all blobs)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, SasIpRange, UserDelegationKey, resource::blob::{Container, ContainerPermissions}};
//! use std::net::Ipv4Addr;
//! use time::OffsetDateTime;
//! use time::macros::datetime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: datetime!(2025-05-21 00:00:00 UTC),
//! #     signed_expiry: datetime!(2025-05-22 00:00:00 UTC),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: vec![0; 32],
//! # };
//! let sas = SasBuilder::new("myaccount", Container::new("logs"),
//!     ContainerPermissions { read: true, list: true, ..Default::default() },
//!     OffsetDateTime::now_utc() + time::Duration::hours(4), &udk)
//!     .ip_range(SasIpRange::Range {
//!         start: Ipv4Addr::new(10, 0, 0, 1).into(),
//!         end: Ipv4Addr::new(10, 0, 0, 255).into(),
//!     });
//!
//! let query = sas.to_string();
//! // => "sv=2025-11-05&sr=c&...&sip=10.0.0.1-10.0.0.255&sp=rl&..."
//! ```
//!
//! ## Directory SAS (ADLS Gen2)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, UserDelegationKey, resource::blob::{Directory, ContainerPermissions}};
//! use time::OffsetDateTime;
//! use time::macros::datetime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: datetime!(2025-05-21 00:00:00 UTC),
//! #     signed_expiry: datetime!(2025-05-22 00:00:00 UTC),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: vec![0; 32],
//! # };
//! // Depth is computed automatically from the path (2 segments here)
//! let sas = SasBuilder::new("myaccount", Directory::new("filesystem", "path/to"),
//!     ContainerPermissions { read: true, list: true, ..Default::default() },
//!     OffsetDateTime::now_utc() + time::Duration::hours(1), &udk);
//!
//! let query = sas.to_string();
//! // => "sv=2025-11-05&sr=d&...&sdd=2&..."
//! ```
//!
//! ## Queue SAS (read + process messages)
//!
//! ```rust
//! use azure_storage_sas::{SasBuilder, SasProtocol, UserDelegationKey, resource::{Queue, QueuePermissions}};
//! use time::OffsetDateTime;
//! use time::macros::datetime;
//!
//! # let udk = UserDelegationKey {
//! #     signed_oid: "oid".into(), signed_tid: "tid".into(),
//! #     signed_start: datetime!(2025-05-21 00:00:00 UTC),
//! #     signed_expiry: datetime!(2025-05-22 00:00:00 UTC),
//! #     signed_service: "b".into(), signed_version: "2025-11-05".into(),
//! #     value: vec![0; 32],
//! # };
//! let sas = SasBuilder::new("myaccount", Queue::new("work-items"),
//!     QueuePermissions { read: true, process: true, ..Default::default() },
//!     OffsetDateTime::now_utc() + time::Duration::hours(8), &udk)
//!     .protocol(SasProtocol::Https)
//!     .delegated_tenant_id("tenant-id");
//!
//! let query = sas.to_string();
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
