// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::{
    generated::clients::{
        BlobClient as GeneratedBlobClient, HierarchicalClient as GeneratedHierarchicalClient,
        HierarchicalClientOptions,
    },
    models::{
        HierarchicalClientAppendOptions, HierarchicalClientCreateOptions,
        HierarchicalClientDownloadOptions, HierarchicalClientFlushOptions,
        HierarchicalClientRenameOptions, HierarchicalClientSetAccessControlOptions,
    },
    pipeline::StorageHeadersPolicy,
    BlobClient, BlobClientOptions,
};
use azure_core::{
    credentials::TokenCredential,
    http::{
        policies::{BearerTokenCredentialPolicy, Policy},
        NoFormat, RawResponse, Request, RequestContent, Response, Url,
    },
    Bytes, Result,
};
use std::marker::PhantomData;
use std::sync::Arc;

/// Marker types for type state, TODO: Export elsewhere
pub struct File;
pub struct Directory;

// Struct, use struct initializer to get top-level, No State client
pub struct HierarchicalClient<T> {
    pub(crate) endpoint: Url,
    pub(crate) client: GeneratedHierarchicalClient,
    pub(crate) _marker: PhantomData<T>,
}

// Generic type, shared functionality
impl<T> HierarchicalClient<T> {
    pub fn blob_client(&self) -> BlobClient {
        BlobClient {
            endpoint: self.client.endpoint.clone(),
            client: GeneratedBlobClient {
                blob_name: self.client.blob_name.clone(),
                container_name: self.client.container_name.clone(),
                endpoint: self.client.endpoint.clone(),
                pipeline: self.client.pipeline.clone(),
                version: self.client.version.clone(),
            },
        }
    }
}

// Conversion methods from No State -> State
impl HierarchicalClient<()> {
    pub fn file(self) -> HierarchicalClient<File> {
        HierarchicalClient {
            endpoint: self.endpoint.clone(),
            client: self.client,
            _marker: PhantomData::<File>,
        }
    }

    pub fn directory(self) -> HierarchicalClient<Directory> {
        HierarchicalClient {
            endpoint: self.endpoint.clone(),
            client: self.client,
            _marker: PhantomData::<Directory>,
        }
    }
}

// File state specific functions
impl HierarchicalClient<File> {
    pub async fn create(
        &self,
        options: Option<HierarchicalClientCreateOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.create("file".to_string(), options).await
    }

    pub async fn append_data(
        &self,
        data: RequestContent<Bytes>,
        offset: i64,
        length: i64,
        options: Option<HierarchicalClientAppendOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.append_data(data, offset, length, options).await
    }

    pub async fn flush_data(
        &self,
        offset: i64,
        options: Option<HierarchicalClientFlushOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.flush_data(offset, options).await
    }

    pub async fn set_access_control(
        &self,
        options: Option<HierarchicalClientSetAccessControlOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.set_access_control(options).await
    }
}

// Directory state specific functions
impl HierarchicalClient<Directory> {
    pub async fn create(
        &self,
        options: Option<HierarchicalClientCreateOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.create("directory".to_string(), options).await
    }

    pub async fn rename_directory(
        &self,
        new_name: String,
        options: Option<HierarchicalClientRenameOptions<'_>>,
    ) -> Result<RawResponse> {
        self.client.rename_directory(new_name, options).await
    }

    // Get a FileClient under current directory
    pub fn file_client(&self, file_name: String) -> HierarchicalClient<File> {
        let file_name = format!("{}/{}", self.client.blob_name, file_name);
        let generated_client = GeneratedHierarchicalClient {
            blob_name: file_name,
            container_name: self.client.container_name.clone(),
            endpoint: self.client.endpoint.clone(),
            pipeline: self.client.pipeline.clone(),
            version: self.client.version.clone(),
        };
        HierarchicalClient {
            endpoint: self.endpoint.clone(),
            client: generated_client,
            _marker: PhantomData::<File>,
        }
    }

    pub fn sub_directory(&self, blob_name: String) -> HierarchicalClient<Directory> {
        let blob_name = format!("{}/{}", self.client.blob_name, blob_name);
        let generated_client = GeneratedHierarchicalClient {
            blob_name,
            container_name: self.client.container_name.clone(),
            endpoint: self.client.endpoint.clone(),
            pipeline: self.client.pipeline.clone(),
            version: self.client.version.clone(),
        };
        HierarchicalClient {
            endpoint: self.endpoint.clone(),
            client: generated_client,
            _marker: PhantomData::<Directory>,
        }
    }
}
