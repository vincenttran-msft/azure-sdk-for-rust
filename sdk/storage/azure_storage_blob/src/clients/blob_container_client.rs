// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::policies::storage_headers_policy::StorageHeadersPolicy;
use azure_core::credentials::TokenCredential;
use azure_core::headers::HeaderName;
use azure_core::{
    AsClientOptions, BearerTokenCredentialPolicy, Context, Method, Policy, Request, Response,
    Result, Url,
};
use azure_identity::DefaultAzureCredentialBuilder;
use blob_storage::blob_client::BlobClientOptions;
use blob_storage::blob_container::{
    BlobContainer, BlobContainerCreateOptions, BlobContainerGetAccountInfoOptions,
    BlobContainerGetPropertiesOptions,
};
use blob_storage::BlobClient as GeneratedBlobClient;
use std::sync::Arc;
use uuid::Uuid;
pub struct ContainerClient {
    endpoint: String,
    container_name: String,
    credential: Option<Arc<dyn TokenCredential>>,
    client: GeneratedBlobClient,
}

impl ContainerClient {
    const VERSION_ID: &'static str = ("2024-08-04");

    pub fn new(
        endpoint: String,
        container_name: String,
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
            container_name: container_name.clone(),
            credential,
            client: client,
        })
    }

    pub async fn create_container(
        &self,
        options: Option<BlobContainerCreateOptions<'_>>,
    ) -> Result<Response<()>> {
        self.client
            .get_blob_container_client()
            .create(
                self.container_name.clone(),
                String::from(Self::VERSION_ID), //svc version
                Some(BlobContainerCreateOptions::default()),
            )
            .await
    }

    pub async fn get_container_properties(
        &self,
        options: Option<BlobContainerGetPropertiesOptions<'_>>,
    ) -> Result<Response<()>> {
        self.client
            .get_blob_container_client()
            .get_properties(
                self.container_name.clone(),
                String::from(Self::VERSION_ID), //svc version
                Some(BlobContainerGetPropertiesOptions::default()),
            )
            .await
    }

    pub async fn get_account_info(
        &self,
        options: Option<BlobContainerGetAccountInfoOptions<'_>>,
    ) -> Result<Response<()>> {
        self.client
            .get_blob_container_client()
            .get_account_info(
                self.container_name.clone(),
                String::from(Self::VERSION_ID), //svc version
                Some(BlobContainerGetAccountInfoOptions::default()),
            )
            .await
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    // Don't forget to az-login
    async fn test_get_container_properties_auth() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let container_client = ContainerClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            String::from("acontainer108f32e8"),
            Some(credential),
            Some(BlobClientOptions::default()),
        )
        .unwrap();
        let response = container_client
            .get_container_properties(Some(BlobContainerGetPropertiesOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }

    // #[tokio::test]
    // Don't forget to az-login
    // This fails for kind: HttpResponse { status: LengthRequired, error_code: None
    // async fn test_create_container() {
    //     let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
    //     let container_client = ContainerClient::new(
    //         String::from("https://vincenttranstock.blob.core.windows.net/"),
    //         String::from("mynewcontainer"),
    //         Some(credential),
    //         Some(BlobClientOptions::default()),
    //     )
    //     .unwrap();
    //     let response = container_client
    //         .create_container(Some(BlobContainerCreateOptions::default()))
    //         .await
    //         .unwrap();
    //     print!("{:?}", response);
    //     print!(
    //         "\n{:?}",
    //         response.into_body().collect_string().await.unwrap()
    //     );
    // }
    #[tokio::test]
    // Don't forget to az-login
    async fn test_get_account_info_auth() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let container_client = ContainerClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            String::from("acontainer108f32e8"),
            Some(credential),
            Some(BlobClientOptions::default()),
        )
        .unwrap();
        let response = container_client
            .get_account_info(Some(BlobContainerGetAccountInfoOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }
}
