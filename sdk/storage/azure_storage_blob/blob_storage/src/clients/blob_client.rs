// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::blob_blob::{BlobBlobDownloadOptions, BlobBlobGetPropertiesOptions};
use crate::blob_client::BlobClientOptions;
use crate::clients::units::*;
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

pub struct BlobClient<BlobType = Unset> {
    pub(crate) blob_type: PhantomData<BlobType>,
    pub(crate) endpoint: String,
    pub(crate) container_name: String,
    pub(crate) blob_name: String,
    pub(crate) credential: Option<Arc<dyn TokenCredential>>,
    pub(crate) client: GeneratedBlobClient,
}

impl BlobClient<Unset> {
    const VERSION_ID: &'static str = ("2024-08-04");

    pub fn new(
        endpoint: String,
        container_name: String,
        blob_name: String,
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
            blob_type: PhantomData::<Unset>,
            endpoint: endpoint.clone(),
            container_name: container_name.clone(),
            blob_name: blob_name.clone(),
            credential,
            client: client,
        })
    }

    pub async fn as_append_blob(&self) -> BlobClient<Append> {
        BlobClient {
            blob_type: PhantomData::<Append>,
            endpoint: self.endpoint.clone(),
            container_name: self.container_name.clone(),
            blob_name: self.blob_name.clone(),
            credential: self.credential.clone(),
            client: GeneratedBlobClient {
                endpoint: self.client.endpoint.clone(),
                pipeline: self.client.pipeline.clone(),
            },
        }
    }

    pub async fn download_blob(
        &self,
        options: Option<BlobBlobDownloadOptions<'_>>,
    ) -> Result<Response<Vec<u8>>> {
        // This hard-coded value still works, even though this is technically a bug for version_id
        let version = String::from("80bc3c5e-3bb7-95f6-6c57-8ceb2c9155");
        self.client
            .get_blob_blob_client()
            .download(
                self.container_name.clone(),
                self.blob_name.clone(),
                version,                        //blob version
                String::from(Self::VERSION_ID), //svc version
                options,
            )
            .await
    }

    pub async fn get_blob_properties(
        &self,
        options: Option<BlobBlobGetPropertiesOptions<'_>>,
    ) -> Result<Response<()>> {
        // This hard-coded value still works, even though this is technically a bug for version_id
        let version = String::from("80bc3c5e-3bb7-95f6-6c57-8ceb2c9155");
        self.client
            .get_blob_blob_client()
            .get_properties(
                self.container_name.clone(),
                self.blob_name.clone(),
                version,                        //blob version
                String::from(Self::VERSION_ID), //svc version
                Some(BlobBlobGetPropertiesOptions::default()),
            )
            .await
    }
}

impl<Append> BlobClient<Append> {
    pub async fn append_block(&self) {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_blob() {
        let blob_client = BlobClient::new(
            String::from("https://vincenttranpublicac.blob.core.windows.net/"),
            String::from("public"),
            String::from("hello.txt"),
            None,
            Some(BlobClientOptions::default()),
        )
        .unwrap();
        let response = blob_client
            .download_blob(Some(BlobBlobDownloadOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_download_blob_if_tags_match() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let blob_client = BlobClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            String::from("options-bag-testing"),
            String::from("i_have_tags.txt"),
            Some(credential),
            Some(BlobClientOptions::default()),
        )
        .unwrap();

        // Build an BlobBlobDownloadOptions that contains if_tags_match with the matching condition to  {tagged: yes}
        // These are expected as: "<key>"='<value'. but need to be unicode encoded
        let if_tags = String::from("\u{0022}tagged\u{0022}=\u{0027}yes\u{0027}");
        let download_options_builder = BlobBlobDownloadOptions::builder().with_if_tags(if_tags);
        let mut download_options = download_options_builder.build();
        let response = blob_client
            .download_blob(Some(download_options))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_blob_properties() {
        let blob_client = BlobClient::new(
            String::from("https://vincenttranpublicac.blob.core.windows.net/"),
            String::from("public"),
            String::from("hello.txt"),
            None,
            Some(BlobClientOptions::default()),
        )
        .unwrap();
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

    #[tokio::test]
    // Don't forget to az-login
    async fn test_download_blob_authenticated() {
        let credential = DefaultAzureCredentialBuilder::default().build().unwrap();
        let blob_client = BlobClient::new(
            String::from("https://vincenttranstock.blob.core.windows.net/"),
            String::from("acontainer108f32e8"),
            String::from("hello.txt"),
            Some(credential),
            Some(BlobClientOptions::default()),
        )
        .unwrap();
        let response = blob_client
            .download_blob(Some(BlobBlobDownloadOptions::default()))
            .await
            .unwrap();
        print!("{:?}", response);
        print!(
            "\n{:?}",
            response.into_body().collect_string().await.unwrap()
        );
    }

    #[tokio::test]
    async fn test_get_append_client() {
        let blob_client = BlobClient::new(
            String::from("https://vincenttranpublicac.blob.core.windows.net/"),
            String::from("public"),
            String::from("hello.txt"),
            None,
            Some(BlobClientOptions::default()),
        )
        .unwrap();
        let append_block_client = blob_client.as_append_blob().await;
        append_block_client.append_block();
    }
}
