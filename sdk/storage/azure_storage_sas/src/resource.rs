// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Resource types that can be protected by a SAS token.

pub mod blob;

mod account;
mod queue;

pub use account::{Account, AccountPermissions, AccountResourceTypes, AccountServices};
pub use queue::{Queue, QueuePermissions};

use crate::builder::Fields;
use std::fmt;

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// A storage resource type for SAS generation.
///
/// This trait is sealed and cannot be implemented outside this crate.
pub trait Resource: sealed::Sealed {
    /// The permissions type for this resource (e.g., `BlobPermissions`).
    type Permissions: fmt::Display;

    /// The signing context required to build the string-to-sign.
    ///
    /// For account SAS this is `()` (account key signing is external).
    /// For user delegation SAS this is [`UserDelegationKey`](crate::UserDelegationKey).
    type SigningContext;

    #[doc(hidden)]
    fn _build_string_to_sign(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
    ) -> String;

    #[doc(hidden)]
    fn _build_query_parameters(
        &self,
        permissions: &Self::Permissions,
        fields: &Fields,
        context: &Self::SigningContext,
        signature: &str,
    ) -> String;
}

/// Marker trait for blob-service resources.
///
/// Types implementing this trait support response header overrides and
/// other blob-service-specific SAS fields.
pub trait BlobServiceResource: Resource {}

/// Marker trait for resources that use a user delegation key.
pub trait DelegatedResource: Resource<SigningContext = crate::UserDelegationKey> {}
