#![allow(clippy::module_inception)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::ptr_arg)]
#![allow(clippy::large_enum_variant)]
#![allow(clippy::derive_partial_eq_without_eq)]
#[cfg(feature = "package-2021-02-16-preview")]
pub mod package_2021_02_16_preview;
#[cfg(all(feature = "package-2021-02-16-preview", not(feature = "no-default-tag")))]
pub use package_2021_02_16_preview::*;