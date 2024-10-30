// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::{Context, Method, Request, Response, Result, Url};
use blob_storage::blob_blob::BlobBlobDownloadOptions;
use blob_storage::blob_client::BlobClientOptions;
use blob_storage::BlobClient as GeneratedBlobClient;
use std::sync::Arc;
use uuid::Uuid;

// Later this will be auto-populated with current version, otherwise could take user input as well
// const CURRENT_SVC_VERSION: String = String::from("2024-08-04");

pub struct BlobClient {
    account_name: String,
    container_name: String,
    blob_name: String,
    client: GeneratedBlobClient,
}

impl BlobClient {
    pub fn new(
        account_name: String,
        container_name: String,
        blob_name: String,
        options: Option<BlobClientOptions>,
    ) -> Result<Self> {
        // Build Blob URL from input
        let endpoint = "https://".to_owned() + &account_name + ".blob.core.windows.net/";
        let options = options.unwrap_or_default();
        let client = GeneratedBlobClient::with_no_credential(endpoint, Some(options))?;

        Ok(Self {
            account_name: account_name.clone(),
            container_name: container_name.clone(),
            blob_name: blob_name.clone(),
            client: client,
        })
    }

    // fn build_url(account_name: &str) -> String {
    //     "https://".to_owned() + account_name + ".blob.core.windows.net/"
    // }

    pub async fn download_blob(
        &self,
        options: Option<BlobBlobDownloadOptions<'_>>, // Curious if this is the right move, or if we can do a simple wrapper with an easy Into convert to the generated version
    ) -> Result<Response<Vec<u8>>> {
        //TODO: Inject version through a pipeline policy
        let version = Uuid::new_v4().to_string();
        self.client
            .get_blob_blob_client()
            .download(
                self.container_name.clone(),
                self.blob_name.clone(),
                version,
                String::from("2024-08-04"),
                Some(BlobBlobDownloadOptions::default()),
            )
            .await
    }

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
            String::from("vincenttranpublicac"),
            String::from("public"),
            String::from("hello.txt"),
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
