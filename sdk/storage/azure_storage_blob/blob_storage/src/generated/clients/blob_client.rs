// Copyright (c) Microsoft Corporation. All rights reserved.
//
// Licensed under the MIT License. See License.txt in the project root for license information.
// Code generated by Microsoft (R) Rust Code Generator. DO NOT EDIT.

use crate::blob_append_blob::BlobAppendBlob;
use crate::blob_blob::BlobBlob;
use crate::blob_block_blob::BlobBlockBlob;
use crate::blob_container::BlobContainer;
use crate::blob_page_blob::BlobPageBlob;
use crate::blob_service::BlobService;
use azure_core::builders::ClientOptionsBuilder;
use azure_core::{ClientOptions, Pipeline, Policy, Result, RetryOptions, TransportOptions, Url};
use std::sync::Arc;

pub struct BlobClient {
    endpoint: Url,
    pipeline: Pipeline,
}

#[derive(Clone, Debug)]
pub struct BlobClientOptions {
    client_options: ClientOptions,
}

impl BlobClient {
    pub fn with_no_credential(
        endpoint: impl AsRef<str>,
        options: Option<BlobClientOptions>,
    ) -> Result<Self> {
        let mut endpoint = Url::parse(endpoint.as_ref())?;
        endpoint.query_pairs_mut().clear();
        let options = options.unwrap_or_default();
        Ok(Self {
            endpoint,
            pipeline: Pipeline::new(
                option_env!("CARGO_PKG_NAME"),
                option_env!("CARGO_PKG_VERSION"),
                options.client_options,
                Vec::default(),
                Vec::default(),
            ),
        })
    }

    pub fn get_blob_append_blob_client(&self) -> BlobAppendBlob {
        BlobAppendBlob {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }

    pub fn get_blob_blob_client(&self) -> BlobBlob {
        BlobBlob {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }

    pub fn get_blob_block_blob_client(&self) -> BlobBlockBlob {
        BlobBlockBlob {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }

    pub fn get_blob_container_client(&self) -> BlobContainer {
        BlobContainer {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }

    pub fn get_blob_page_blob_client(&self) -> BlobPageBlob {
        BlobPageBlob {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }

    pub fn get_blob_service_client(&self) -> BlobService {
        BlobService {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
        }
    }
}

impl BlobClientOptions {
    pub fn builder() -> builders::BlobClientOptionsBuilder {
        builders::BlobClientOptionsBuilder::new()
    }
}

impl Default for BlobClientOptions {
    fn default() -> Self {
        Self {
            client_options: ClientOptions::default(),
        }
    }
}

impl ClientOptionsBuilder for BlobClientOptions {
    fn with_per_call_policies<P>(mut self, per_call_policies: P) -> Self
    where
        P: Into<Vec<Arc<dyn Policy>>>,
        Self: Sized,
    {
        self.client_options.set_per_call_policies(per_call_policies);
        self
    }

    fn with_per_try_policies<P>(mut self, per_try_policies: P) -> Self
    where
        P: Into<Vec<Arc<dyn Policy>>>,
        Self: Sized,
    {
        self.client_options.set_per_try_policies(per_try_policies);
        self
    }

    fn with_retry<P>(mut self, retry: P) -> Self
    where
        P: Into<RetryOptions>,
        Self: Sized,
    {
        self.client_options.set_retry(retry);
        self
    }

    fn with_transport<P>(mut self, transport: P) -> Self
    where
        P: Into<TransportOptions>,
        Self: Sized,
    {
        self.client_options.set_transport(transport);
        self
    }
}

pub mod builders {
    use super::*;

    pub struct BlobClientOptionsBuilder {
        options: BlobClientOptions,
    }

    impl BlobClientOptionsBuilder {
        pub(super) fn new() -> Self {
            Self {
                options: BlobClientOptions::default(),
            }
        }

        pub fn build(&self) -> BlobClientOptions {
            self.options.clone()
        }
    }
}
