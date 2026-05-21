// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub use crate::generated::clients::{BlobServiceClient, BlobServiceClientOptions};

use crate::{
    models::{KeyInfo, UserDelegationKey},
    BlobClient, BlobContainerClient,
};
use azure_core::{
    credentials::TokenCredential,
    error::CheckSuccessOptions,
    fmt::SafeDebug,
    http::{
        policies::{auth::BearerTokenAuthorizationPolicy, Policy},
        ClientMethodOptions, Method, Pipeline, PipelineSendOptions, Request, Response, Url, UrlExt,
        XmlFormat,
    },
    time::{to_rfc3339, OffsetDateTime},
    tracing, xml, Result,
};
use std::sync::Arc;

/// Options to be passed to [`BlobServiceClient::get_user_delegation_key`].
#[derive(Clone, Default, SafeDebug)]
pub struct BlobServiceClientGetUserDelegationKeyOptions<'a> {
    /// Allows customization of the method call.
    pub method_options: ClientMethodOptions<'a>,

    /// The timeout parameter, expressed in seconds. See
    /// [Setting Timeouts for Blob Service Operations](https://learn.microsoft.com/rest/api/storageservices/setting-timeouts-for-blob-service-operations).
    pub timeout: Option<i32>,
}

impl BlobServiceClient {
    /// Creates a new BlobServiceClient from a service URL.
    ///
    /// # Arguments
    ///
    /// * `service_url` - The full URL of the Azure storage account, for example `https://myaccount.blob.core.windows.net/`.
    ///   The caller is responsible for percent-encoding the URL correctly; it will be used as-is.
    /// * `credential` - An optional implementation of [`TokenCredential`] that can provide an Entra ID token to use when authenticating.
    /// * `options` - Optional configuration for the client.
    #[tracing::new("Storage.Blob.Service")]
    pub fn new(
        service_url: Url,
        credential: Option<Arc<dyn TokenCredential>>,
        options: Option<BlobServiceClientOptions>,
    ) -> Result<Self> {
        // Storage endpoints must be base URLs.
        if service_url.cannot_be_a_base() {
            return Err(azure_core::Error::with_message(
                azure_core::error::ErrorKind::Other,
                format!("{service_url} is not a valid base URL"),
            ));
        }
        let mut options = options.unwrap_or_default();
        super::apply_client_defaults(&mut options.client_options);

        let mut per_retry_policies: Vec<Arc<dyn Policy>> = Vec::default();
        if let Some(token_credential) = credential {
            if !service_url.scheme().starts_with("https") {
                return Err(azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Other,
                    format!("{service_url} must use https"),
                ));
            }
            per_retry_policies.push(Arc::new(BearerTokenAuthorizationPolicy::new(
                token_credential,
                vec!["https://storage.azure.com/.default"],
            )));
        }

        let pipeline = Pipeline::new(
            option_env!("CARGO_PKG_NAME"),
            option_env!("CARGO_PKG_VERSION"),
            options.client_options.clone(),
            Vec::default(),
            per_retry_policies,
            None,
        );

        Ok(Self {
            endpoint: service_url,
            version: options.version,
            pipeline,
        })
    }

    /// Returns a new instance of BlobContainerClient.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container.
    pub fn blob_container_client(&self, container_name: &str) -> BlobContainerClient {
        let mut container_url = self.url().clone();
        container_url
            .path_segments_mut()
            // This should not fail as service URL has already been validated on client construction.
            .expect("Cannot be a base URL.")
            .push(container_name);

        BlobContainerClient {
            endpoint: container_url,
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        }
    }

    /// Returns a new instance of BlobClient.
    ///
    /// # Arguments
    ///
    /// * `container_name` - The name of the container.
    /// * `blob_name` - The name of the blob.
    pub fn blob_client(&self, container_name: &str, blob_name: &str) -> BlobClient {
        let mut blob_url = self.url().clone();
        blob_url
            .path_segments_mut()
            // This should not fail as service URL has already been validated on client construction.
            .expect("Cannot be a base URL.")
            .extend([container_name, blob_name]);

        BlobClient {
            endpoint: blob_url,
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        }
    }

    /// Gets the URL of the resource this client is configured for.
    pub fn url(&self) -> &Url {
        &self.endpoint
    }

    /// Requests a user delegation key from the service.
    ///
    /// The returned key contains secret material used to sign
    /// [user delegation SAS](https://learn.microsoft.com/rest/api/storageservices/create-user-delegation-sas)
    /// tokens. The client must be authenticated with a [`TokenCredential`].
    ///
    /// # Arguments
    ///
    /// * `start` - The time at which the key becomes valid. Sub-second precision is dropped.
    /// * `expiry` - The time at which the key expires (maximum 7 days from `start`). Sub-second precision is dropped.
    /// * `options` - Optional parameters for the request.
    #[tracing::function("Storage.Blob.BlobServiceClient.getUserDelegationKey")]
    pub async fn get_user_delegation_key(
        &self,
        start: OffsetDateTime,
        expiry: OffsetDateTime,
        options: Option<BlobServiceClientGetUserDelegationKeyOptions<'_>>,
    ) -> Result<Response<UserDelegationKey, XmlFormat>> {
        let options = options.unwrap_or_default();
        let ctx = options.method_options.context.to_borrowed();
        let mut url = self.endpoint.clone();
        let mut query_builder = url.query_builder();
        query_builder
            .append_pair("comp", "userdelegationkey")
            .append_pair("restype", "service");
        if let Some(timeout) = options.timeout {
            query_builder.set_pair("timeout", timeout.to_string());
        }
        query_builder.build();
        let start = start.replace_nanosecond(0).unwrap_or(start);
        let expiry = expiry.replace_nanosecond(0).unwrap_or(expiry);
        let body = xml::to_xml(&KeyInfo {
            start: to_rfc3339(&start),
            expiry: to_rfc3339(&expiry),
        })?;
        let mut request = Request::new(url, Method::Post);
        request.insert_header("accept", "application/xml");
        request.insert_header("content-type", "application/xml");
        request.insert_header("x-ms-version", &self.version);
        request.set_body(body);
        let rsp = self
            .pipeline
            .send(
                &ctx,
                &mut request,
                Some(PipelineSendOptions {
                    check_success: CheckSuccessOptions {
                        success_codes: &[200],
                    },
                    ..Default::default()
                }),
            )
            .await?;
        Ok(rsp.into())
    }
}
