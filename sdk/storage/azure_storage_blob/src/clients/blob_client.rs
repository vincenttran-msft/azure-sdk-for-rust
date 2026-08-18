// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub use crate::generated::clients::{BlobClient, BlobClientOptions};

use crate::{
    generated::{
        clients::BlobClient as GeneratedBlobClient,
        models::{BlobClientDownloadInternalOptions, BlobClientListLayoutOptions},
    },
    models::{
        BlobClientDownloadIntoResult, BlobClientDownloadOptions, BlobClientDownloadResult,
        BlobClientUploadOptions, BlobClientUploadResult, BlobDownloadProperties, HttpRange,
        LayoutAwareRouting, StorageErrorCode,
    },
    partitioned_transfer::{self, Layout, LayoutEndpoint, PartitionedDownloadBehavior},
    AppendBlobClient, BlockBlobClient, PageBlobClient,
};
use async_trait::async_trait;
use azure_core::{
    async_runtime::get_async_runtime,
    credentials::TokenCredential,
    error::ErrorKind,
    http::{
        headers::Headers,
        policies::{auth::BearerTokenAuthorizationPolicy, Policy},
        AsyncRawResponse, Context, Etag, NoFormat, Pipeline, RequestContent, StatusCode, Url,
        UrlExt,
    },
    tracing, Bytes, Result,
};
use futures::lock::Mutex;
use std::{
    ops::Range,
    sync::{Arc, OnceLock},
    time::{Duration, Instant},
};

impl BlobClient {
    /// Creates a new BlobClient from a blob URL.
    ///
    /// # Arguments
    ///
    /// * `blob_url` - The full URL of the blob, for example `https://myaccount.blob.core.windows.net/mycontainer/myblob`.
    ///   The caller is responsible for percent-encoding the URL correctly; it will be used as-is.
    /// * `credential` - An optional implementation of [`TokenCredential`] that can provide an Entra ID token to use when authenticating.
    /// * `options` - Optional configuration for the client.
    #[tracing::new("Storage.Blob.Blob")]
    pub fn new(
        blob_url: Url,
        credential: Option<Arc<dyn TokenCredential>>,
        options: Option<BlobClientOptions>,
    ) -> Result<Self> {
        // Storage endpoints must be base URLs.
        if blob_url.cannot_be_a_base() {
            return Err(azure_core::Error::with_message(
                azure_core::error::ErrorKind::Other,
                format!("{blob_url} is not a valid base URL"),
            ));
        }

        let mut options = options.unwrap_or_default();
        super::apply_client_defaults(&mut options.client_options);

        let mut per_retry_policies: Vec<Arc<dyn Policy>> = Vec::default();
        if let Some(token_credential) = credential {
            if !blob_url.scheme().starts_with("https") {
                return Err(azure_core::Error::with_message(
                    azure_core::error::ErrorKind::Other,
                    format!("{blob_url} must use https"),
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
            vec![Arc::new(partitioned_transfer::LayoutRoutingPolicy)],
            per_retry_policies,
            None,
        );

        Ok(Self {
            endpoint: blob_url,
            version: options.version,
            pipeline,
        })
    }

    /// Returns a new instance of AppendBlobClient.
    pub fn append_blob_client(&self) -> AppendBlobClient {
        AppendBlobClient {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        }
    }

    /// Returns a new instance of BlockBlobClient.
    pub fn block_blob_client(&self) -> BlockBlobClient {
        BlockBlobClient {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        }
    }

    /// Returns a new instance of PageBlobClient.
    pub fn page_blob_client(&self) -> PageBlobClient {
        PageBlobClient {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        }
    }

    /// Gets the URL of the resource this client is configured for.
    pub fn url(&self) -> &Url {
        &self.endpoint
    }

    /// Creates a new BlobClient targeting a specific blob version.
    ///
    /// # Arguments
    ///
    /// * `version_id` - The version ID of the blob to target.
    pub fn with_version(&self, version_id: &str) -> Result<Self> {
        let mut versioned_endpoint = self.endpoint.clone();
        {
            let mut query_builder = versioned_endpoint.query_builder();
            query_builder.set_pair("versionid", version_id);
            query_builder.build();
        }

        Ok(Self {
            endpoint: versioned_endpoint,
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        })
    }

    /// Creates a new BlobClient targeting a specific blob snapshot.
    ///
    /// # Arguments
    ///
    /// * `snapshot` - The snapshot ID of the blob to target.
    pub fn with_snapshot(&self, snapshot: &str) -> Result<Self> {
        let mut snapshot_endpoint = self.endpoint.clone();
        {
            let mut query_builder = snapshot_endpoint.query_builder();
            query_builder.set_pair("snapshot", snapshot);
            query_builder.build();
        }

        Ok(Self {
            endpoint: snapshot_endpoint,
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        })
    }

    /// Downloads a blob and its contents from the service.
    ///
    /// This operation performs a managed (multi-part) download, splitting the blob into
    /// parallel range requests for better performance on large blobs. The returned
    /// [`BlobClientDownloadResult::body`] contains the complete blob data, while
    /// [`BlobClientDownloadResult::properties`] and [`BlobClientDownloadResult::headers`]
    /// reflect only the initial response's metadata and properties.
    ///
    /// If the streamed bytes of this blob are to be collected into contiguous memory,
    /// consider instead calling [`BlobClient::download_into`] with a pre-allocated buffer
    /// to avoid unnecessary copies and allocations.
    ///
    /// # Arguments
    ///
    /// * `options` - Optional configuration for the request.
    ///
    /// # Notes
    ///
    /// By default, storage clients create their HTTP transport via
    /// [`azure_core::http::new_http_client()`] with automatic decompression disabled.
    /// If you set a custom transport in [`BlobClientOptions`] without also disabling
    /// automatic decompression, partitioned downloads may not succeed.
    #[tracing::function("Storage.Blob.Blob.download")]
    pub async fn download(
        &self,
        options: Option<BlobClientDownloadOptions<'_>>,
    ) -> Result<BlobClientDownloadResult> {
        let options = options.unwrap_or_default();
        let parallel = options
            .parallel
            .unwrap_or_else(crate::partitioned_transfer::defaults::default_concurrency);
        let partition_size = options
            .partition_size
            .unwrap_or(crate::partitioned_transfer::defaults::DEFAULT_DOWNLOAD_PARTITION_SIZE);
        let inner_client = GeneratedBlobClient {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        };
        let layout_request = layout_request_from_options(&options);
        let range = options.range.clone();
        let behavior =
            BlobClientDownloadBehavior::new(inner_client, options.into(), layout_request);
        let response =
            partitioned_transfer::download(range, parallel, partition_size, Arc::new(behavior))
                .await?;
        BlobClientDownloadResult::from_headers(response)
    }

    /// Downloads a blob and its contents from the service.
    ///
    /// This operation performs a managed (multi-part) download, splitting the blob into
    /// parallel range requests for better performance on large blobs. The downloaded bytes are
    /// written directly into the provided `buffer`.
    ///
    /// # Arguments
    ///
    /// * `buffer` - Destination buffer to write the downloaded blob data into.
    /// * `options` - Optional configuration for the request.
    ///
    /// # Notes
    ///
    /// By default, storage clients create their HTTP transport via
    /// [`azure_core::http::new_http_client()`] with automatic decompression disabled.
    /// If you set a custom transport in [`BlobClientOptions`] without also disabling
    /// automatic decompression, partitioned downloads may not succeed.
    #[tracing::function("Storage.Blob.Blob.download_into")]
    pub async fn download_into(
        &self,
        buffer: &mut [u8],
        options: Option<BlobClientDownloadOptions<'_>>,
    ) -> Result<BlobClientDownloadIntoResult> {
        let options = options.unwrap_or_default();
        let parallel = options
            .parallel
            .unwrap_or_else(crate::partitioned_transfer::defaults::default_concurrency);
        let partition_size = options
            .partition_size
            .unwrap_or(crate::partitioned_transfer::defaults::DEFAULT_DOWNLOAD_PARTITION_SIZE);
        let inner_client = GeneratedBlobClient {
            endpoint: self.endpoint.clone(),
            pipeline: self.pipeline.clone(),
            version: self.version.clone(),
            tracer: self.tracer.clone(),
        };
        let layout_request = layout_request_from_options(&options);
        let range = options.range.clone();
        let behavior =
            BlobClientDownloadBehavior::new(inner_client, options.into(), layout_request);
        let (_, headers, len) = partitioned_transfer::download_into(
            buffer,
            range,
            parallel,
            partition_size,
            Arc::new(behavior),
        )
        .await?;
        Ok(BlobClientDownloadIntoResult {
            len,
            properties: BlobDownloadProperties::from_headers(&headers)?,
            headers,
        })
    }

    /// Uploads content to a block blob, overwriting any existing blob by default.
    ///
    /// Updating an existing block blob overwrites any existing metadata on the blob. Use [`BlobClientUploadOptions::if_not_exists()`] to fail instead of overwriting.
    /// To perform a partial update of the content of a block blob, use [`BlockBlobClient::stage_block()`] and [`BlockBlobClient::commit_block_list()`] directly.
    ///
    /// # Arguments
    ///
    /// * `content` - The content to upload.
    /// * `options` - Optional parameters for the request.
    pub async fn upload(
        &self,
        content: RequestContent<Bytes, NoFormat>,
        options: Option<BlobClientUploadOptions<'_>>,
    ) -> Result<BlobClientUploadResult> {
        self.block_blob_client().upload(content, options).await
    }

    /// Checks if the blob exists.
    ///
    /// Returns `true` if the blob exists, `false` if the blob does not exist, and propagates all other errors.
    pub async fn exists(&self) -> Result<bool> {
        match self.get_properties(None).await {
            Ok(_) => Ok(true),
            Err(e) if e.http_status() == Some(StatusCode::NotFound) => match e.kind() {
                ErrorKind::HttpResponse {
                    error_code: Some(error_code),
                    ..
                } if error_code == StorageErrorCode::BlobNotFound.as_ref()
                    || error_code == StorageErrorCode::ContainerNotFound.as_ref() =>
                {
                    Ok(false)
                }
                // Propagate all other error types.
                _ => Err(e),
            },
            Err(e) => Err(e),
        }
    }
}

struct BlobClientDownloadBehavior<'a> {
    client: GeneratedBlobClient,
    options: BlobClientDownloadInternalOptions<'a>,
    /// Get Blob Layout request template, present only when routing is enabled; used
    /// by [`prepare`](PartitionedDownloadBehavior::prepare) to fetch the layout when
    /// the service hints at one.
    layout_request: Option<BlobClientListLayoutOptions<'static>>,
    /// The refreshing layout for this download, populated once by `prepare` before any
    /// subsequent chunk is issued. `Some(None)` means no routing; unset means the
    /// initial chunk, which is never routed.
    cache: OnceLock<Option<LayoutCache>>,
}

impl<'a> BlobClientDownloadBehavior<'a> {
    fn new(
        client: GeneratedBlobClient,
        options: BlobClientDownloadInternalOptions<'a>,
        layout_request: Option<BlobClientListLayoutOptions<'static>>,
    ) -> Self {
        Self {
            client,
            options,
            layout_request,
            cache: OnceLock::new(),
        }
    }
}

#[async_trait]
impl PartitionedDownloadBehavior for BlobClientDownloadBehavior<'_> {
    async fn transfer_range(
        &self,
        range: Option<Range<usize>>,
        etag_lock: Option<Etag>,
    ) -> Result<AsyncRawResponse> {
        let mut opt = self.options.clone();
        // Route this chunk to the endpoint serving its start offset, if a layout is
        // available; the per-call policy rewrites the request to that endpoint.
        if let Some(Some(cache)) = self.cache.get() {
            if let Some(range) = &range {
                if let Some(layout) = cache.current(&self.client).await {
                    if let Some(endpoint) = layout.ideal_endpoint(range.start as i64) {
                        opt.method_options.context = opt
                            .method_options
                            .context
                            .clone()
                            .with_value(LayoutEndpoint(endpoint.to_owned()));
                    }
                }
            }
        }
        opt.range = range.map(HttpRange::from);
        if let Some(etag) = etag_lock {
            opt.if_match = Some(etag);
            opt.if_none_match = None;
            opt.if_modified_since = None;
            opt.if_unmodified_since = None;
            opt.if_tags = None;
        }
        self.client
            .download_internal(Some(opt))
            .await
            .map(AsyncRawResponse::from)
    }

    async fn prepare(&self, initial_headers: &Headers, etag_lock: Option<&Etag>) -> Result<()> {
        let Some(layout_request) = &self.layout_request else {
            return Ok(()); // routing disabled
        };
        // Only fetch the layout when the service recommends it for this blob.
        if initial_headers.get_optional_str(&"x-ms-download-hint".into()) != Some("layout") {
            let _ = self.cache.set(None);
            return Ok(());
        }
        let mut request = layout_request.clone();
        // Pin the layout to the version the download is already reading.
        if request.if_match.is_none() {
            request.if_match = etag_lock.cloned();
        }
        let context = self.options.method_options.context.clone();
        let cache = partitioned_transfer::fetch_layout(&self.client, &context, &request)
            .await?
            .map(|prefetch| LayoutCache::new(request, Arc::new(prefetch.layout)));
        let _ = self.cache.set(cache);
        Ok(())
    }
}

/// Server-side layout lifetime; a cached layout must not be used to route past this.
const LAYOUT_TTL: Duration = Duration::from_secs(300);
/// How long before expiry a proactive background refresh is started.
const LAYOUT_REFRESH_BUFFER: Duration = Duration::from_secs(30);
/// How long to wait after a failed refresh before trying again.
const LAYOUT_REFRESH_BACKOFF: Duration = Duration::from_secs(30);

/// A single download's refreshing view of the blob layout.
///
/// Routing uses the cached layout while it is valid. Once past the refresh buffer a
/// single-flight refresh runs in the background without blocking the download; once past
/// expiry, routing is suspended (falls back to the account) until a refresh succeeds. The
/// layout is only a hint, so refreshes are best-effort — downloaded bytes stay correct
/// regardless, because every request is pinned to the initial response's ETag.
struct LayoutCache {
    /// ETag-pinned Get Blob Layout request, reused verbatim for each refresh.
    request: BlobClientListLayoutOptions<'static>,
    state: Arc<Mutex<CachedLayout>>,
}

struct CachedLayout {
    layout: Arc<Layout>,
    /// When a proactive background refresh should start (expiry minus the refresh buffer).
    refresh_at: Instant,
    /// When the layout is no longer safe to route with; past this, routing is suspended.
    expires_at: Instant,
    /// `true` while a background refresh is in flight (single-flight guard).
    refreshing: bool,
    /// Earliest time to retry after a failed refresh; `None` when not backing off.
    retry_at: Option<Instant>,
}

impl CachedLayout {
    fn new(layout: Arc<Layout>) -> Self {
        let now = Instant::now();
        Self {
            layout,
            refresh_at: now + (LAYOUT_TTL - LAYOUT_REFRESH_BUFFER),
            expires_at: now + LAYOUT_TTL,
            refreshing: false,
            retry_at: None,
        }
    }
}

impl LayoutCache {
    fn new(request: BlobClientListLayoutOptions<'static>, layout: Arc<Layout>) -> Self {
        Self {
            request,
            state: Arc::new(Mutex::new(CachedLayout::new(layout))),
        }
    }

    /// Returns the layout to route this chunk with, or `None` to fall back to the account.
    ///
    /// Never blocks on the network: once past the refresh buffer it starts a single-flight
    /// background refresh and keeps serving the current layout; once past expiry it returns
    /// `None` until a refresh completes.
    async fn current(&self, client: &GeneratedBlobClient) -> Option<Arc<Layout>> {
        let mut state = self.state.lock().await;
        let now = Instant::now();
        let backing_off = matches!(state.retry_at, Some(at) if now < at);
        if now >= state.refresh_at && !state.refreshing && !backing_off {
            state.refreshing = true;
            // Detach the refresh; it runs to completion independently of this handle.
            let _refresh = get_async_runtime().spawn(Box::pin(Self::refresh(
                clone_blob_client(client),
                self.request.clone(),
                self.state.clone(),
            )));
        }
        (now < state.expires_at).then(|| state.layout.clone())
    }

    /// Fetches a fresh layout and updates `state`. On success the layout and its refresh and
    /// expiry deadlines are reset; on failure the previous layout is kept (routed only while
    /// still unexpired) and a backoff is applied before the next attempt.
    async fn refresh(
        client: GeneratedBlobClient,
        request: BlobClientListLayoutOptions<'static>,
        state: Arc<Mutex<CachedLayout>>,
    ) {
        let result = partitioned_transfer::fetch_layout(&client, &Context::new(), &request).await;
        let mut state = state.lock().await;
        match result {
            Ok(Some(prefetch)) => {
                *state = CachedLayout::new(Arc::new(prefetch.layout));
            }
            Ok(None) | Err(_) => {
                state.refreshing = false;
                state.retry_at = Some(Instant::now() + LAYOUT_REFRESH_BACKOFF);
            }
        }
    }
}

/// Reconstructs the generated client so a background refresh can own it; the generated
/// `BlobClient` is not `Clone`, but its fields are.
fn clone_blob_client(client: &GeneratedBlobClient) -> GeneratedBlobClient {
    GeneratedBlobClient {
        endpoint: client.endpoint.clone(),
        pipeline: client.pipeline.clone(),
        version: client.version.clone(),
        tracer: client.tracer.clone(),
    }
}

/// Builds the Get Blob Layout request template used to fetch the layout, or `None`
/// when locality-aware routing is not enabled for this download.
fn layout_request_from_options(
    options: &BlobClientDownloadOptions<'_>,
) -> Option<BlobClientListLayoutOptions<'static>> {
    matches!(options.layout_aware_routing, LayoutAwareRouting::Enabled)
        .then(|| layout_options_from_download(options))
}

/// Maps download options to the equivalent Get Blob Layout options, carrying the
/// range, conditions, and customer-provided-key material used by the download.
fn layout_options_from_download(
    options: &BlobClientDownloadOptions<'_>,
) -> BlobClientListLayoutOptions<'static> {
    BlobClientListLayoutOptions {
        encryption_algorithm: options.encryption_algorithm,
        encryption_key: options.encryption_key.clone(),
        encryption_key_sha256: options.encryption_key_sha256.clone(),
        if_match: options.if_match.clone(),
        if_modified_since: options.if_modified_since,
        if_none_match: options.if_none_match.clone(),
        if_tags: options.if_tags.clone(),
        if_unmodified_since: options.if_unmodified_since,
        lease_id: options.lease_id.clone(),
        range: options.range.clone(),
        snapshot: options.snapshot.clone(),
        timeout: options.timeout,
        version_id: options.version_id.clone(),
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use azure_core::http::{ClientOptions, FixedRetryOptions, HttpClient, RetryOptions, Transport};
    use azure_core_test::http::MockHttpClient;
    use futures::FutureExt as _;
    use std::sync::atomic::{AtomicUsize, Ordering};

    const LAYOUT_V1: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<BlobLayout>
  <Endpoints>
    <Endpoint Index="0" Value="epv1.blob.storage.azure.net:443" />
  </Endpoints>
  <Ranges>
    <Range Start="0" End="8388607" EndpointIndex="0" />
  </Ranges>
</BlobLayout>"#;

    const LAYOUT_V2: &[u8] = br#"<?xml version="1.0" encoding="utf-8"?>
<BlobLayout>
  <Endpoints>
    <Endpoint Index="0" Value="epv2.blob.storage.azure.net:443" />
  </Endpoints>
  <Ranges>
    <Range Start="0" End="8388607" EndpointIndex="0" />
  </Ranges>
</BlobLayout>"#;

    fn mock_client(transport: Arc<dyn HttpClient>) -> GeneratedBlobClient {
        BlobClient::new(
            "https://acct.blob.core.windows.net/container/blob"
                .parse()
                .unwrap(),
            None,
            Some(BlobClientOptions {
                client_options: ClientOptions {
                    transport: Some(Transport::new(transport)),
                    // Keep failure-path tests fast: don't retry mocked 5xx responses.
                    retry: RetryOptions::fixed(FixedRetryOptions {
                        max_retries: 0,
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                ..Default::default()
            }),
        )
        .unwrap()
    }

    fn etag_headers() -> Headers {
        let mut headers = Headers::new();
        headers.insert("etag", "etag-1".to_owned());
        headers
    }

    async fn seed_layout(
        client: &GeneratedBlobClient,
        request: &BlobClientListLayoutOptions<'static>,
    ) -> Layout {
        partitioned_transfer::fetch_layout(client, &Context::new(), request)
            .await
            .unwrap()
            .unwrap()
            .layout
    }

    #[tokio::test]
    async fn refresh_success_updates_layout_and_resets_deadlines() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mock: Arc<dyn HttpClient> = Arc::new(MockHttpClient::new(move |_req| {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move {
                let body = if n == 0 { LAYOUT_V1 } else { LAYOUT_V2 };
                Ok(AsyncRawResponse::from_bytes(
                    StatusCode::Ok,
                    etag_headers(),
                    Bytes::from_static(body),
                ))
            }
            .boxed()
        }));
        let client = mock_client(mock);
        let request = BlobClientListLayoutOptions::default();

        // Seed with v1 (fetch #1) in a mid-refresh state.
        let state = Arc::new(Mutex::new(CachedLayout {
            layout: Arc::new(seed_layout(&client, &request).await),
            refresh_at: Instant::now(),
            expires_at: Instant::now(),
            refreshing: true,
            retry_at: None,
        }));

        // Refresh (fetch #2) swaps to v2 and resets the deadlines.
        LayoutCache::refresh(client, request, state.clone()).await;
        let state = state.lock().await;
        assert_eq!(
            state.layout.ideal_endpoint(0),
            Some("epv2.blob.storage.azure.net:443")
        );
        assert!(!state.refreshing);
        assert!(state.retry_at.is_none());
        assert!(state.expires_at > Instant::now());
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn current_routes_while_valid_and_suspends_once_expired() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mock: Arc<dyn HttpClient> = Arc::new(MockHttpClient::new(move |_req| {
            counter.fetch_add(1, Ordering::SeqCst);
            async move {
                Ok(AsyncRawResponse::from_bytes(
                    StatusCode::Ok,
                    etag_headers(),
                    Bytes::from_static(LAYOUT_V1),
                ))
            }
            .boxed()
        }));
        let client = mock_client(mock);
        let request = BlobClientListLayoutOptions::default();
        let cache = LayoutCache::new(
            request.clone(),
            Arc::new(seed_layout(&client, &request).await),
        );

        // Valid, before the refresh window: routes, no extra fetch.
        assert_eq!(
            cache.current(&client).await.unwrap().ideal_endpoint(0),
            Some("epv1.blob.storage.azure.net:443")
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1);

        // Expired and backing off: routing is suspended and no refresh is started.
        {
            let mut state = cache.state.lock().await;
            let now = Instant::now();
            state.refresh_at = now - Duration::from_secs(1);
            state.expires_at = now - Duration::from_secs(1);
            state.retry_at = Some(now + Duration::from_secs(300));
        }
        assert!(cache.current(&client).await.is_none());
        assert_eq!(calls.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn current_refreshes_in_background_without_blocking() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mock: Arc<dyn HttpClient> = Arc::new(MockHttpClient::new(move |_req| {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move {
                let body = if n == 0 { LAYOUT_V1 } else { LAYOUT_V2 };
                Ok(AsyncRawResponse::from_bytes(
                    StatusCode::Ok,
                    etag_headers(),
                    Bytes::from_static(body),
                ))
            }
            .boxed()
        }));
        let client = mock_client(mock);
        let request = BlobClientListLayoutOptions::default();
        let cache = LayoutCache::new(
            request.clone(),
            Arc::new(seed_layout(&client, &request).await),
        );

        // Enter the refresh window: past refresh_at but still valid.
        {
            let mut state = cache.state.lock().await;
            let now = Instant::now();
            state.refresh_at = now - Duration::from_secs(1);
            state.expires_at = now + Duration::from_secs(100);
        }

        // Returns the current layout immediately (does not block on the refresh)...
        assert_eq!(
            cache.current(&client).await.unwrap().ideal_endpoint(0),
            Some("epv1.blob.storage.azure.net:443")
        );

        // ...while a single-flight background refresh swaps in v2.
        let mut spins = 0;
        loop {
            {
                let state = cache.state.lock().await;
                if !state.refreshing
                    && state.layout.ideal_endpoint(0) == Some("epv2.blob.storage.azure.net:443")
                {
                    assert!(state.expires_at > Instant::now() + Duration::from_secs(200));
                    assert!(state.retry_at.is_none());
                    break;
                }
            }
            assert!(spins < 10_000, "background refresh did not complete");
            spins += 1;
            tokio::task::yield_now().await;
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn refresh_failure_keeps_layout_and_backs_off() {
        let calls = Arc::new(AtomicUsize::new(0));
        let counter = calls.clone();
        let mock: Arc<dyn HttpClient> = Arc::new(MockHttpClient::new(move |_req| {
            let n = counter.fetch_add(1, Ordering::SeqCst);
            async move {
                if n == 0 {
                    Ok(AsyncRawResponse::from_bytes(
                        StatusCode::Ok,
                        etag_headers(),
                        Bytes::from_static(LAYOUT_V1),
                    ))
                } else {
                    Ok(AsyncRawResponse::from_bytes(
                        StatusCode::InternalServerError,
                        Headers::new(),
                        Bytes::from_static(b""),
                    ))
                }
            }
            .boxed()
        }));
        let client = mock_client(mock);
        let request = BlobClientListLayoutOptions::default();

        // Seed with v1 and an expiry still in the future.
        let expires_at = Instant::now() + Duration::from_secs(100);
        let state = Arc::new(Mutex::new(CachedLayout {
            layout: Arc::new(seed_layout(&client, &request).await),
            refresh_at: Instant::now(),
            expires_at,
            refreshing: true,
            retry_at: None,
        }));

        // The failed refresh (5xx) keeps v1 and applies a backoff without moving expiry.
        LayoutCache::refresh(client, request, state.clone()).await;
        let state = state.lock().await;
        assert_eq!(
            state.layout.ideal_endpoint(0),
            Some("epv1.blob.storage.azure.net:443")
        );
        assert!(!state.refreshing);
        assert_eq!(state.expires_at, expires_at);
        match state.retry_at {
            Some(retry_at) => assert!(retry_at > Instant::now()),
            None => panic!("expected a backoff to be set"),
        }
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }
}
