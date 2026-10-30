// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Clients used to communicate with Azure Blob Storage.

use azure_core::http::{new_http_client, ClientOptions, HttpClientOptions, Transport, Url};

use crate::logging::apply_storage_logging_defaults;

mod append_blob_client;
mod blob_client;
mod blob_container_client;
mod blob_service_client;
mod block_blob_client;
mod page_blob_client;

pub use append_blob_client::{AppendBlobClient, AppendBlobClientOptions};
pub use blob_client::{BlobClient, BlobClientOptions};
pub use blob_container_client::{BlobContainerClient, BlobContainerClientOptions};
pub use blob_service_client::{BlobServiceClient, BlobServiceClientOptions};
pub use block_blob_client::{BlockBlobClient, BlockBlobClientOptions};
pub use page_blob_client::{PageBlobClient, PageBlobClientOptions};

fn is_local_http_endpoint(url: &Url) -> bool {
    url.scheme() == "http" && matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"))
}

#[allow(clippy::needless_update)]
fn apply_client_defaults(options: &mut ClientOptions) {
    if options.transport.is_none() {
        options.transport = Some(Transport::new(new_http_client(Some(HttpClientOptions {
            automatic_decompression: false,
            ..Default::default()
        }))))
    }
    apply_storage_logging_defaults(options);
}

#[cfg(test)]
mod tests {
    use super::is_local_http_endpoint;
    use azure_core::http::Url;

    #[test]
    fn local_http_endpoints() {
        for endpoint in [
            "http://localhost:10000",
            "http://127.0.0.1:10000",
            "http://[::1]:10000",
        ] {
            assert!(is_local_http_endpoint(&Url::parse(endpoint).unwrap()));
        }
        for endpoint in [
            "http://example.com",
            "http://192.168.1.1",
            "http://localhost.example.com",
            "http://127.0.0.1.example.com",
            "ftp://localhost",
            "https://localhost",
        ] {
            assert!(!is_local_http_endpoint(&Url::parse(endpoint).unwrap()));
        }
    }
}
