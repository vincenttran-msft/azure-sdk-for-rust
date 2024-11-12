// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::blob_blob::BlobBlobGetPropertiesOptions;
use crate::blob_client::BlobClientOptions;
use crate::blob_container::{
    BlobContainer, BlobContainerCreateOptions, BlobContainerGetAccountInfoOptions,
    BlobContainerGetPropertiesOptions,
};
use crate::blob_service::BlobServiceGetPropertiesOptions;
use crate::clients::units::*;
use crate::models::StorageServiceProperties;
use crate::policies::storage_headers_policy::StorageHeadersPolicy;
use crate::BlobClient as GeneratedBlobClient;
use azure_core::credentials::TokenCredential;
use azure_core::headers::HeaderName;
use azure_core::{
    AsClientOptions, BearerTokenCredentialPolicy, Context, Method, Policy, Request, Response,
    Result, Url,
};
use azure_identity::DefaultAzureCredentialBuilder;
use std::marker::PhantomData;
use std::sync::Arc;
use uuid::Uuid;

use super::blob_client::BlobClient;
pub struct BlobServiceClient {
    endpoint: String,
    credential: Option<Arc<dyn TokenCredential>>,
    client: GeneratedBlobClient,
}

impl BlobServiceClient {
    const VERSION_ID: &'static str = ("2024-08-04");

    pub fn new(
        endpoint: String,
        credential: Option<Arc<dyn TokenCredential>>,
        options: Option<BlobClientOptions>,
    ) -> Result<Self> {
        let mut options = BlobClientOptions::default();

        // Fold in StorageHeadersPolicy policy via ClientOptions
        let mut client_options = options.client_options.clone();
        let mut per_call_policies = client_options.per_call_policies().clone();
        let storage_headers_policy = Arc::new(StorageHeadersPolicy::new());
        per_call_policies.push(storage_headers_policy);
        client_options.set_per_call_policies(per_call_policies);

        // Conditionally add authentication if provided
        if credential.is_some() {
            let oauth_token_policy = BearerTokenCredentialPolicy::new(
                credential.clone().unwrap(),
                ["https://storage.azure.com/.default"],
            );
            let mut per_try_policies = client_options.per_call_policies().clone();
            per_try_policies.push(Arc::new(oauth_token_policy) as Arc<dyn Policy>);
            client_options.set_per_try_policies(per_try_policies);
        }

        // Set it after modifying everything
        options.client_options = client_options.clone();

        let client =
            GeneratedBlobClient::with_no_credential(endpoint.clone(), Some(options.clone()))?;

        Ok(Self {
            endpoint: endpoint.clone(),
            credential,
            client: client,
        })
    }

    pub async fn get_service_properties(
        &self,
        options: Option<BlobServiceGetPropertiesOptions<'_>>,
    ) -> Result<Response<StorageServiceProperties>> {
        self.client
            .get_blob_service_client()
            .get_properties(
                String::from(Self::VERSION_ID), //svc version
                Some(BlobServiceGetPropertiesOptions::default()),
            )
            .await
    }

    pub fn get_blob_client(
        &self,
        container_name: String,
        blob_name: String,
        options: Option<BlobClientOptions>,
    ) -> BlobClient {
        BlobClient {
            blob_type: PhantomData::<Unset>,
            endpoint: self.client.endpoint.clone().to_string(),
            container_name: container_name,
            blob_name: blob_name,
            credential: self.credential.clone(),
            client: GeneratedBlobClient {
                endpoint: self.client.endpoint.clone(),
                pipeline: self.client.pipeline.clone(),
            },
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    // Also fails for ContainerClientNotFound
    // Don't forget to az-login
    async fn test_get_service_properties() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let service_client = BlobServiceClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            Some(credential),
            Some(BlobClientOptions::default()),
        )
        .unwrap();

        let response = service_client
            .get_service_properties(Some(BlobServiceGetPropertiesOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }

    #[tokio::test]
    // Don't forget to az-login
    async fn test_get_blob_client() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let service_client = BlobServiceClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            Some(credential.clone()),
            Some(BlobClientOptions::default()),
        )
        .unwrap();

        let blob_client = service_client.get_blob_client(
            String::from("acontainer108f32e8"),
            String::from("hello.txt"),
            Some(BlobClientOptions::default()),
        );

        let response = blob_client
            .get_blob_properties(Some(BlobBlobGetPropertiesOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }
}
