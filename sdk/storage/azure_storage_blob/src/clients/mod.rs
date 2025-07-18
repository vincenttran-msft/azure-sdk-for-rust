// Copyright (c) Microsoft Corporation. All rights reserved.
//
// Licensed under the MIT License. See License.txt in the project root for license information.
// Code generated by Microsoft (R) Rust Code Generator. DO NOT EDIT.

mod append_blob_client;
mod blob_client;
mod blob_container_client;
mod blob_service_client;
mod block_blob_client;
mod page_blob_client;

pub use append_blob_client::AppendBlobClient;
pub use blob_client::BlobClient;
pub use blob_container_client::BlobContainerClient;
pub use blob_service_client::BlobServiceClient;
pub use block_blob_client::BlockBlobClient;
pub use page_blob_client::PageBlobClient;

pub use crate::generated::clients::{
    AppendBlobClientOptions, BlobClientOptions, BlobContainerClientOptions,
    BlobServiceClientOptions, BlockBlobClientOptions, PageBlobClientOptions,
};
