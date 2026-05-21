// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]

mod blob;
mod common;
mod ip_range;
mod key;
mod protocol;

pub use blob::{
    BlobContainerSasBuilder, BlobContainerSasPermissions, BlobSasBuilder, BlobSasPermissions,
    BlobSasQueryParameters,
};
pub use ip_range::SasIpRange;
pub use key::UserDelegationKey;
pub use protocol::SasProtocol;

/// Type-state marker indicating that a SAS builder has no signing key set yet.
#[derive(Debug, Default)]
pub struct NoKey;

/// Type-state marker indicating that a SAS builder has a signing key set.
#[derive(Debug)]
pub struct WithKey(pub(crate) UserDelegationKey);

/// The service version emitted as the `sv` query parameter for all SAS tokens
/// produced by this crate.
///
/// This is intentionally not configurable. To change service versions, upgrade
/// to a newer release of `azure_storage_sas`.
pub const SAS_VERSION: &str = "2026-04-06";

/// Returns the SAS service version emitted by this crate.
pub fn sas_version() -> &'static str {
    SAS_VERSION
}
