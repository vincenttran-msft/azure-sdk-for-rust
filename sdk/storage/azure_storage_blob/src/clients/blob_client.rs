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
use blob_storage::blob_blob::{BlobBlobDownloadOptions, BlobBlobGetPropertiesOptions};
use blob_storage::blob_client::BlobClientOptions;
use blob_storage::BlobClient as GeneratedBlobClient;
use std::sync::Arc;
use uuid::Uuid;

pub struct BlobClient {
    endpoint: String,
    container_name: String,
    blob_name: String,
    credential: Option<Arc<dyn TokenCredential>>,
    client: GeneratedBlobClient,
}

impl BlobClient {
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
            endpoint: endpoint.clone(),
            container_name: container_name.clone(),
            blob_name: blob_name.clone(),
            credential,
            client: client,
        })
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
                Some(BlobBlobDownloadOptions::default()),
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

    // pub fn get_container_client(&self) ->

    // pub async fn get_blob_properties(&self) -> Result<Response> {
    //     // Build the get properties request itself
    //     let mut request = Request::new(self.url.to_owned(), Method::Head); // This is technically cloning
    //     BlobClient::finalize_request(&mut request);

    //     // Send the request
    //     let response = self.pipeline.send(&(Context::new()), &mut request).await?;
    //     println!("Response headers: {:?}", response);

    //     // Return the entire response for now
    //     Ok(response)
    // }
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
}

//     #[tokio::test]
//     async fn test_get_blob_properties() {
//         let credential = DefaultAzureCredentialBuilder::default()
//             .build()
//             .map(|cred| Arc::new(cred) as Arc<dyn TokenCredential>)
//             .expect("Failed to build credential");

//         // Create a Blob Client
//         let my_blob_client = BlobClient::new(
//             String::from("vincenttranstock"),
//             String::from("acontainer108f32e8"),
//             String::from("hello.txt"),
//             credential,
//             None,
//         );

//         // Get response
//         let ret = my_blob_client
//             .get_blob_properties()
//             .await
//             .expect("Request failed!");
//         let (status_code, headers, _response_body) = ret.deconstruct();

//         // Assert equality
//         assert_eq!(status_code, azure_core::StatusCode::Ok);
//         assert_eq!(
//             headers
//                 .get_str(&HeaderName::from_static("content-length"))
//                 .expect("Failed getting content-length header"),
//             "10"
//         )
//     }
// }
