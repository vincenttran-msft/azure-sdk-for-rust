// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub use crate::generated::clients::{PageBlobClient, PageBlobClientOptions};

use azure_core::{
    credentials::TokenCredential,
    http::{Pipeline, Url},
    tracing, Result,
};
use std::sync::Arc;

impl PageBlobClient {
    /// Creates a new PageBlobClient from a blob URL.
    ///
    /// # Arguments
    ///
    /// * `blob_url` - The full URL of the Page blob, for example `https://myaccount.blob.core.windows.net/mycontainer/myblob`.
    ///   The caller is responsible for percent-encoding the URL correctly; it will be used as-is.
    /// * `credential` - An optional implementation of [`TokenCredential`] that can provide an Entra ID token to use when authenticating.
    /// * `options` - Optional configuration for the client.
    #[tracing::new("Storage.Blob.PageBlob")]
    pub fn new(
        blob_url: Url,
        credential: Option<Arc<dyn TokenCredential>>,
        options: Option<PageBlobClientOptions>,
    ) -> Result<Self> {
        // Storage endpoints must be base URLs.
        if blob_url.cannot_be_a_base() {
            return Err(azure_core::Error::with_message(
                azure_core::error::ErrorKind::Other,
                format!("{blob_url} is not a valid base URL"),
            ));
        }

        let mut options = options.unwrap_or_default();
        let session_options = options.session_options.clone().unwrap_or_default();
        // Build auth policies from the pre-default options so the session provider's
        // own service client applies its defaults exactly once.
        let per_retry_policies = super::build_auth_policies(
            &blob_url,
            credential,
            &session_options,
            &options.client_options,
            &options.version,
        )?;
        super::apply_client_defaults(&mut options.client_options);

        let pipeline = Pipeline::new(
            option_env!("CARGO_PKG_NAME"),
            option_env!("CARGO_PKG_VERSION"),
            options.client_options.clone(),
            Vec::default(),
            per_retry_policies,
            None,
        );

        Ok(Self {
            endpoint: blob_url,
            pipeline,
            session_options: options.session_options,
            version: options.version,
        })
    }

    /// Gets the URL of the blob.
    pub fn url(&self) -> &Url {
        &self.endpoint
    }
}
