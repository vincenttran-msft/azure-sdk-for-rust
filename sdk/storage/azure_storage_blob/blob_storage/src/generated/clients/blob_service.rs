// Copyright (c) Microsoft Corporation. All rights reserved.
//
// Licensed under the MIT License. See License.txt in the project root for license information.
// Code generated by Microsoft (R) Rust Code Generator. DO NOT EDIT.

use crate::models::{
    FilterBlobSegment, FilterBlobsIncludes, KeyInfo, ListContainersSegmentResponse,
    StorageServiceProperties, StorageServiceStats, UserDelegationKey,
};
use azure_core::builders::ClientMethodOptionsBuilder;
use azure_core::{
    AsClientMethodOptions, ClientMethodOptions, Context, Method, Pipeline, Request, RequestContent,
    Response, Result, Url,
};

pub struct BlobService {
    pub(in crate::generated::clients) endpoint: Url,
    pub(in crate::generated::clients) pipeline: Pipeline,
}

impl BlobService {
    /// The Filter Blobs operation enables callers to list blobs across all containers whose tags match a given search expression.
    pub async fn filter_blobs(
        &self,
        version: impl Into<String>,
        options: Option<BlobServiceFilterBlobsOptions<'_>>,
    ) -> Result<Response<FilterBlobSegment>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?comp=blobs");
        if let Some(include) = options.include {
            url.query_pairs_mut().append_pair(
                "include",
                &include
                    .iter()
                    .map(|i| i.to_string())
                    .collect::<Vec<String>>()
                    .join(","),
            );
        }
        if let Some(marker) = options.marker {
            url.query_pairs_mut().append_pair("marker", &marker);
        }
        if let Some(maxresults) = options.maxresults {
            url.query_pairs_mut()
                .append_pair("maxresults", &maxresults.to_string());
        }
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        if let Some(where_param) = options.where_param {
            url.query_pairs_mut().append_pair("where", &where_param);
        }
        let mut request = Request::new(url, Method::Get);
        request.insert_header("accept", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// Returns the sku name and account kind.
    pub async fn get_account_info(
        &self,
        version: impl Into<String>,
        options: Option<BlobServiceGetAccountInfoOptions<'_>>,
    ) -> Result<Response<()>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?restype=account&comp=properties");
        let mut request = Request::new(url, Method::Get);
        request.insert_header("accept", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// Retrieves properties of a storage account's Blob service, including properties for Storage Analytics and CORS (Cross-Origin
    /// Resource Sharing) rules.
    pub async fn get_properties(
        &self,
        version: impl Into<String>,
        options: Option<BlobServiceGetPropertiesOptions<'_>>,
    ) -> Result<Response<StorageServiceProperties>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?restype=service&comp=properties");
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Get);
        request.insert_header("accept", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// Retrieves statistics related to replication for the Blob service. It is only available on the secondary location endpoint
    /// when read-access geo-redundant replication is enabled for the storage account.
    pub async fn get_statistics(
        &self,
        version: impl Into<String>,
        options: Option<BlobServiceGetStatisticsOptions<'_>>,
    ) -> Result<Response<StorageServiceStats>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?restype=service&comp=stats");
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Get);
        request.insert_header("accept", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// The Get User Delegation Key operation gets the user delegation key for the Blob service. This is only a valid operation
    /// when using User Delegation SAS. For more information, see <a href=\"https://docs.microsoft.com/en-us/rest/api/storageservices/create-user-delegation-sas\">Create
    /// a user delegation SAS</a>.
    pub async fn get_user_delegation_key(
        &self,
        body: RequestContent<KeyInfo>,
        version: impl Into<String>,
        options: Option<BlobServiceGetUserDelegationKeyOptions<'_>>,
    ) -> Result<Response<UserDelegationKey>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?restype=service&comp=userdelegationkey");
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Post);
        request.insert_header("accept", "application/json");
        request.insert_header("content-type", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        request.set_body(body);
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// The List Containers Segment operation returns a list of the containers under the specified account
    pub async fn list_containers_segment(
        &self,
        version: impl Into<String>,
        options: Option<BlobServiceListContainersSegmentOptions<'_>>,
    ) -> Result<Response<ListContainersSegmentResponse>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?comp=list");
        if let Some(marker) = options.marker {
            url.query_pairs_mut().append_pair("marker", &marker);
        }
        if let Some(maxresults) = options.maxresults {
            url.query_pairs_mut()
                .append_pair("maxresults", &maxresults.to_string());
        }
        if let Some(prefix) = options.prefix {
            url.query_pairs_mut().append_pair("prefix", &prefix);
        }
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Get);
        request.insert_header("accept", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// Get the properties of a storage account's Blob service, including properties for Storage Analytics and CORS (Cross-Origin
    /// Resource Sharing) rules.
    pub async fn set_properties(
        &self,
        body: RequestContent<StorageServiceProperties>,
        version: impl Into<String>,
        options: Option<BlobServiceSetPropertiesOptions<'_>>,
    ) -> Result<Response<()>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?restype=service&comp=properties");
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Put);
        request.insert_header("accept", "application/json");
        request.insert_header("content-type", "application/json");
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        request.set_body(body);
        self.pipeline.send(&mut ctx, &mut request).await
    }

    /// The Batch operation allows multiple API calls to be embedded into a single HTTP request.
    pub async fn submit_batch(
        &self,
        content_length: i64,
        version: impl Into<String>,
        options: Option<BlobServiceSubmitBatchOptions<'_>>,
    ) -> Result<Response<()>> {
        let options = options.unwrap_or_default();
        let mut ctx = options.method_options.context();
        let mut url = self.endpoint.clone();
        url.set_path("/?comp=batch");
        if let Some(timeout) = options.timeout {
            url.query_pairs_mut()
                .append_pair("timeout", &timeout.to_string());
        }
        let mut request = Request::new(url, Method::Post);
        request.insert_header("accept", "application/json");
        request.insert_header("content-length", content_length.to_string());
        if let Some(request_id) = options.request_id {
            request.insert_header("x-ms-client-request-id", request_id);
        }
        request.insert_header("x-ms-version", version.into());
        self.pipeline.send(&mut ctx, &mut request).await
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceFilterBlobsOptions<'a> {
    include: Option<Vec<FilterBlobsIncludes>>,
    marker: Option<String>,
    maxresults: Option<i32>,
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
    where_param: Option<String>,
}

impl<'a> BlobServiceFilterBlobsOptions<'a> {
    pub fn builder() -> builders::BlobServiceFilterBlobsOptionsBuilder<'a> {
        builders::BlobServiceFilterBlobsOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceGetAccountInfoOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
}

impl<'a> BlobServiceGetAccountInfoOptions<'a> {
    pub fn builder() -> builders::BlobServiceGetAccountInfoOptionsBuilder<'a> {
        builders::BlobServiceGetAccountInfoOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceGetPropertiesOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceGetPropertiesOptions<'a> {
    pub fn builder() -> builders::BlobServiceGetPropertiesOptionsBuilder<'a> {
        builders::BlobServiceGetPropertiesOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceGetStatisticsOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceGetStatisticsOptions<'a> {
    pub fn builder() -> builders::BlobServiceGetStatisticsOptionsBuilder<'a> {
        builders::BlobServiceGetStatisticsOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceGetUserDelegationKeyOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceGetUserDelegationKeyOptions<'a> {
    pub fn builder() -> builders::BlobServiceGetUserDelegationKeyOptionsBuilder<'a> {
        builders::BlobServiceGetUserDelegationKeyOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceListContainersSegmentOptions<'a> {
    marker: Option<String>,
    maxresults: Option<i32>,
    method_options: ClientMethodOptions<'a>,
    prefix: Option<String>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceListContainersSegmentOptions<'a> {
    pub fn builder() -> builders::BlobServiceListContainersSegmentOptionsBuilder<'a> {
        builders::BlobServiceListContainersSegmentOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceSetPropertiesOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceSetPropertiesOptions<'a> {
    pub fn builder() -> builders::BlobServiceSetPropertiesOptionsBuilder<'a> {
        builders::BlobServiceSetPropertiesOptionsBuilder::new()
    }
}

#[derive(Clone, Debug, Default)]
pub struct BlobServiceSubmitBatchOptions<'a> {
    method_options: ClientMethodOptions<'a>,
    request_id: Option<String>,
    timeout: Option<i32>,
}

impl<'a> BlobServiceSubmitBatchOptions<'a> {
    pub fn builder() -> builders::BlobServiceSubmitBatchOptionsBuilder<'a> {
        builders::BlobServiceSubmitBatchOptionsBuilder::new()
    }
}

pub mod builders {
    use super::*;

    pub struct BlobServiceFilterBlobsOptionsBuilder<'a> {
        options: BlobServiceFilterBlobsOptions<'a>,
    }

    impl BlobServiceFilterBlobsOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceFilterBlobsOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceFilterBlobsOptions {
            self.options.clone()
        }

        pub fn with_include(mut self, include: Vec<FilterBlobsIncludes>) -> Self {
            self.options.include = Some(include);
            self
        }

        pub fn with_marker(mut self, marker: String) -> Self {
            self.options.marker = Some(marker);
            self
        }

        pub fn with_maxresults(mut self, maxresults: i32) -> Self {
            self.options.maxresults = Some(maxresults);
            self
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }

        pub fn with_where_param(mut self, where_param: String) -> Self {
            self.options.where_param = Some(where_param);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceFilterBlobsOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceGetAccountInfoOptionsBuilder<'a> {
        options: BlobServiceGetAccountInfoOptions<'a>,
    }

    impl BlobServiceGetAccountInfoOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceGetAccountInfoOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceGetAccountInfoOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceGetAccountInfoOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceGetPropertiesOptionsBuilder<'a> {
        options: BlobServiceGetPropertiesOptions<'a>,
    }

    impl BlobServiceGetPropertiesOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceGetPropertiesOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceGetPropertiesOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceGetPropertiesOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceGetStatisticsOptionsBuilder<'a> {
        options: BlobServiceGetStatisticsOptions<'a>,
    }

    impl BlobServiceGetStatisticsOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceGetStatisticsOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceGetStatisticsOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceGetStatisticsOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceGetUserDelegationKeyOptionsBuilder<'a> {
        options: BlobServiceGetUserDelegationKeyOptions<'a>,
    }

    impl BlobServiceGetUserDelegationKeyOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceGetUserDelegationKeyOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceGetUserDelegationKeyOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceGetUserDelegationKeyOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceListContainersSegmentOptionsBuilder<'a> {
        options: BlobServiceListContainersSegmentOptions<'a>,
    }

    impl BlobServiceListContainersSegmentOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceListContainersSegmentOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceListContainersSegmentOptions {
            self.options.clone()
        }

        pub fn with_marker(mut self, marker: String) -> Self {
            self.options.marker = Some(marker);
            self
        }

        pub fn with_maxresults(mut self, maxresults: i32) -> Self {
            self.options.maxresults = Some(maxresults);
            self
        }

        pub fn with_prefix(mut self, prefix: String) -> Self {
            self.options.prefix = Some(prefix);
            self
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceListContainersSegmentOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceSetPropertiesOptionsBuilder<'a> {
        options: BlobServiceSetPropertiesOptions<'a>,
    }

    impl BlobServiceSetPropertiesOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceSetPropertiesOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceSetPropertiesOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceSetPropertiesOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }

    pub struct BlobServiceSubmitBatchOptionsBuilder<'a> {
        options: BlobServiceSubmitBatchOptions<'a>,
    }

    impl BlobServiceSubmitBatchOptionsBuilder<'_> {
        pub(super) fn new() -> Self {
            Self {
                options: BlobServiceSubmitBatchOptions::default(),
            }
        }

        pub fn build(&self) -> BlobServiceSubmitBatchOptions {
            self.options.clone()
        }

        pub fn with_request_id(mut self, request_id: String) -> Self {
            self.options.request_id = Some(request_id);
            self
        }

        pub fn with_timeout(mut self, timeout: i32) -> Self {
            self.options.timeout = Some(timeout);
            self
        }
    }

    impl<'a> ClientMethodOptionsBuilder<'a> for BlobServiceSubmitBatchOptionsBuilder<'a> {
        fn with_context(mut self, context: &'a Context) -> Self {
            self.options.method_options.set_context(context);
            self
        }
    }
}
