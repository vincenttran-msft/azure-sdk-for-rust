// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Blob layout model and endpoint resolution for locality-aware downloads.
//!
//! The service's Get Blob Layout API describes which byte ranges of a blob are
//! served by which endpoints. This module turns those paginated responses into a
//! flat, ascending list of [`LayoutSegment`]s and resolves the serving endpoint
//! for a given byte offset via binary search.

use std::{collections::HashMap, sync::Arc};

use async_trait::async_trait;
use azure_core::{
    error::ErrorKind,
    http::{
        policies::{Policy, PolicyResult},
        Context, Request, Url,
    },
    Error,
};

use crate::generated::models::BlobLayout;

/// A contiguous byte range of a blob and the endpoint that serves it.
///
/// `end` is the inclusive last byte offset, matching the service's layout ranges.
// Internal-only helper; plain `Debug` is intentional so test assertions can print
// segment contents (endpoints are host:port, not secrets).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LayoutSegment {
    /// Inclusive start byte offset of the range.
    pub start: i64,
    /// Inclusive end byte offset of the range.
    pub end: i64,
    /// The `host:port` endpoint serving this range, or `None` when the service did
    /// not provide one; such ranges download from the client's configured endpoint.
    pub endpoint: Option<String>,
}

/// The resolved layout of a blob: non-overlapping, ascending byte-range segments.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Layout {
    segments: Vec<LayoutSegment>,
}

impl Layout {
    /// Returns `true` when the layout contains no segments (no routing applies).
    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    /// Appends the segments described by a single Get Blob Layout page.
    ///
    /// Endpoint indices are scoped to their page, so each page's `Endpoints` are
    /// mapped independently before its `Ranges` are resolved and appended.
    pub fn extend_from_page(&mut self, page: &BlobLayout) {
        let ranges = &page.ranges.range;
        if ranges.is_empty() {
            return;
        }

        let index_to_endpoint: HashMap<i32, &str> = page
            .endpoints
            .as_ref()
            .and_then(|endpoints| endpoints.endpoint.as_ref())
            .map(|endpoints| {
                endpoints
                    .iter()
                    .filter_map(|endpoint| Some((endpoint.index?, endpoint.value.as_deref()?)))
                    .collect()
            })
            .unwrap_or_default();

        self.segments.reserve(ranges.len());
        for range in ranges {
            let endpoint = range
                .endpoint_index
                .and_then(|index| index_to_endpoint.get(&index).copied())
                .filter(|value| !value.is_empty())
                .map(str::to_owned);
            self.segments.push(LayoutSegment {
                start: range.start.unwrap_or_default(),
                end: range.end.unwrap_or_default(),
                endpoint,
            });
        }
    }

    /// Resolves the serving endpoint for the segment covering `offset`.
    ///
    /// Uses binary search to find the first segment whose inclusive `end` is at or
    /// beyond `offset`. Returns `None` when no segment covers the offset or the
    /// covering segment has no endpoint; callers then fall back to the client's
    /// configured endpoint. The bytes returned are identical regardless.
    pub fn ideal_endpoint(&self, offset: i64) -> Option<&str> {
        if self.segments.is_empty() {
            return None;
        }

        let mut lo: isize = 0;
        let mut hi: isize = self.segments.len() as isize - 1;
        let mut overlap: Option<usize> = None;
        while lo <= hi {
            let mid = lo + (hi - lo) / 2;
            if self.segments[mid as usize].end >= offset {
                overlap = Some(mid as usize);
                hi = mid - 1;
            } else {
                lo = mid + 1;
            }
        }

        overlap.and_then(|index| self.segments[index].endpoint.as_deref())
    }
}

/// Context value carrying the layout endpoint (`host:port` or an absolute URL) that
/// a single download range request should be routed to.
#[derive(Clone, Debug)]
pub(crate) struct LayoutEndpoint(pub String);

/// Per-call pipeline policy that reroutes a request to a layout endpoint while
/// preserving the original account authority as the `Host` header.
///
/// It is a no-op unless a [`LayoutEndpoint`] is present in the request context, so
/// it is safe to install on every request. A malformed endpoint fails the request
/// so that a bad layout response surfaces loudly rather than silently degrading.
#[derive(Debug)]
pub(crate) struct LayoutRoutingPolicy;

#[async_trait]
impl Policy for LayoutRoutingPolicy {
    async fn send(
        &self,
        ctx: &Context,
        request: &mut Request,
        next: &[Arc<dyn Policy>],
    ) -> PolicyResult {
        if let Some(LayoutEndpoint(endpoint)) = ctx.value::<LayoutEndpoint>() {
            apply_layout_endpoint(request, endpoint)?;
        }
        next[0].send(ctx, request, &next[1..]).await
    }
}

/// Rewrites `request`'s URL authority to `endpoint`, preserving the original
/// authority as an explicit `Host` header.
///
/// The rewrite is computed on a scratch copy and only committed once fully
/// successful, so a malformed endpoint returns an error without leaving the
/// request in a half-rewritten state.
fn apply_layout_endpoint(request: &mut Request, endpoint: &str) -> azure_core::Result<()> {
    let (host, port) = parse_endpoint_authority(endpoint).ok_or_else(|| {
        Error::with_message(
            ErrorKind::Other,
            format!("invalid layout endpoint {endpoint:?}"),
        )
    })?;
    let original_host = original_host_header(request.url());
    let mut rewritten = request.url().clone();
    rewritten
        .set_host(Some(&host))
        .map_err(|e| Error::with_error(ErrorKind::Other, e, "invalid layout endpoint host"))?;
    rewritten
        .set_port(port)
        .map_err(|()| Error::with_message(ErrorKind::Other, "invalid layout endpoint port"))?;
    request.insert_header("host", original_host);
    *request.url_mut() = rewritten;
    Ok(())
}

/// Derives the `Host` header value the client would send for `url`, i.e. the host
/// plus the port when a non-default port is explicitly present.
fn original_host_header(url: &Url) -> String {
    match (url.host_str(), url.port()) {
        (Some(host), Some(port)) => format!("{host}:{port}"),
        (Some(host), None) => host.to_owned(),
        (None, _) => String::new(),
    }
}

/// Parses a layout endpoint value into a host and optional port.
///
/// Accepts both the documented `host:port` form and an absolute URL form
/// (`scheme://host[:port]`). Returns `None` for values that cannot be interpreted
/// as an authority; the caller turns that into a failed request.
fn parse_endpoint_authority(endpoint: &str) -> Option<(String, Option<u16>)> {
    let endpoint = endpoint.trim();
    if endpoint.is_empty() {
        return None;
    }

    if endpoint.contains("://") {
        let url = Url::parse(endpoint).ok()?;
        let host = url.host_str()?.to_owned();
        return Some((host, url.port()));
    }

    match endpoint.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => Some((host.to_owned(), Some(port.parse().ok()?))),
        Some(_) => None,
        None => Some((endpoint.to_owned(), None)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generated::models::{
        BlobLayoutEndpoint, BlobLayoutEndpoints, BlobLayoutRange, BlobLayoutRanges,
    };
    use azure_core::http::Method;

    fn segment(start: i64, end: i64, endpoint: Option<&str>) -> LayoutSegment {
        LayoutSegment {
            start,
            end,
            endpoint: endpoint.map(str::to_owned),
        }
    }

    fn layout(segments: Vec<LayoutSegment>) -> Layout {
        Layout { segments }
    }

    fn endpoint(index: i32, value: &str) -> BlobLayoutEndpoint {
        BlobLayoutEndpoint {
            index: Some(index),
            value: Some(value.to_owned()),
        }
    }

    fn range(start: i64, end: i64, endpoint_index: i32) -> BlobLayoutRange {
        BlobLayoutRange {
            start: Some(start),
            end: Some(end),
            endpoint_index: Some(endpoint_index),
        }
    }

    fn page(endpoints: Vec<BlobLayoutEndpoint>, ranges: Vec<BlobLayoutRange>) -> BlobLayout {
        BlobLayout {
            endpoints: Some(BlobLayoutEndpoints {
                endpoint: Some(endpoints),
            }),
            ranges: BlobLayoutRanges { range: ranges },
            ..Default::default()
        }
    }

    #[test]
    fn empty_layout_resolves_to_none() {
        let layout = Layout::default();
        assert!(layout.is_empty());
        assert_eq!(layout.ideal_endpoint(0), None);
        assert_eq!(layout.ideal_endpoint(1_000), None);
    }

    #[test]
    fn single_segment_covers_all_contained_offsets() {
        let layout = layout(vec![segment(0, 99, Some("a"))]);
        assert_eq!(layout.ideal_endpoint(0), Some("a"));
        assert_eq!(layout.ideal_endpoint(50), Some("a"));
        assert_eq!(layout.ideal_endpoint(99), Some("a"));
        // Beyond the final segment's end: no segment covers it.
        assert_eq!(layout.ideal_endpoint(100), None);
    }

    #[test]
    fn binary_search_selects_covering_segment() {
        let layout = layout(vec![
            segment(0, 9, Some("a")),
            segment(10, 19, Some("b")),
            segment(20, 29, Some("c")),
        ]);
        assert_eq!(layout.ideal_endpoint(0), Some("a"));
        assert_eq!(layout.ideal_endpoint(9), Some("a"));
        assert_eq!(layout.ideal_endpoint(10), Some("b"));
        assert_eq!(layout.ideal_endpoint(15), Some("b"));
        assert_eq!(layout.ideal_endpoint(19), Some("b"));
        assert_eq!(layout.ideal_endpoint(20), Some("c"));
        assert_eq!(layout.ideal_endpoint(29), Some("c"));
        assert_eq!(layout.ideal_endpoint(30), None);
    }

    #[test]
    fn inclusive_end_boundaries_route_correctly() {
        let layout = layout(vec![segment(0, 9, Some("a")), segment(10, 19, Some("b"))]);
        // end is inclusive: offset == first segment's end stays on the first segment.
        assert_eq!(layout.ideal_endpoint(9), Some("a"));
        // one past it moves to the next segment.
        assert_eq!(layout.ideal_endpoint(10), Some("b"));
    }

    #[test]
    fn segment_without_endpoint_resolves_to_none() {
        let layout = layout(vec![segment(0, 9, None), segment(10, 19, Some("b"))]);
        assert_eq!(layout.ideal_endpoint(5), None);
        assert_eq!(layout.ideal_endpoint(15), Some("b"));
    }

    #[test]
    fn extend_from_page_maps_endpoint_indices() {
        let mut layout = Layout::default();
        layout.extend_from_page(&page(
            vec![endpoint(0, "h0:443"), endpoint(1, "h1:443")],
            vec![range(0, 9, 1), range(10, 19, 0)],
        ));
        assert_eq!(
            layout.segments,
            vec![
                segment(0, 9, Some("h1:443")),
                segment(10, 19, Some("h0:443")),
            ]
        );
    }

    #[test]
    fn extend_from_page_handles_unordered_endpoint_indices() {
        let mut layout = Layout::default();
        layout.extend_from_page(&page(
            vec![endpoint(2, "h2:443"), endpoint(0, "h0:443")],
            vec![range(0, 9, 2), range(10, 19, 0)],
        ));
        assert_eq!(
            layout.segments,
            vec![
                segment(0, 9, Some("h2:443")),
                segment(10, 19, Some("h0:443")),
            ]
        );
    }

    #[test]
    fn extend_from_page_missing_endpoint_index_yields_none() {
        let mut layout = Layout::default();
        layout.extend_from_page(&page(vec![endpoint(0, "h0:443")], vec![range(0, 9, 7)]));
        assert_eq!(layout.segments, vec![segment(0, 9, None)]);
    }

    #[test]
    fn extend_from_page_accumulates_across_pages_with_independent_indices() {
        let mut layout = Layout::default();
        // Page 1: index 0 -> "p1:443".
        layout.extend_from_page(&page(vec![endpoint(0, "p1:443")], vec![range(0, 9, 0)]));
        // Page 2: index 0 -> "p2:443" (different endpoint space).
        layout.extend_from_page(&page(vec![endpoint(0, "p2:443")], vec![range(10, 19, 0)]));
        assert_eq!(
            layout.segments,
            vec![
                segment(0, 9, Some("p1:443")),
                segment(10, 19, Some("p2:443")),
            ]
        );
    }

    #[test]
    fn extend_from_page_with_empty_ranges_adds_nothing() {
        let mut layout = Layout::default();
        layout.extend_from_page(&page(vec![endpoint(0, "h0:443")], vec![]));
        assert!(layout.is_empty());
    }

    fn request_to(url: &str) -> Request {
        Request::new(url.parse().unwrap(), Method::Get)
    }

    fn host_header(request: &Request) -> Option<String> {
        request
            .headers()
            .get_optional_str(&"host".into())
            .map(str::to_owned)
    }

    #[test]
    fn parse_endpoint_authority_forms() {
        assert_eq!(
            parse_endpoint_authority("host.example.net:443"),
            Some(("host.example.net".to_owned(), Some(443)))
        );
        assert_eq!(
            parse_endpoint_authority("host.example.net:8443"),
            Some(("host.example.net".to_owned(), Some(8443)))
        );
        assert_eq!(
            parse_endpoint_authority("https://host.example.net:8443"),
            Some(("host.example.net".to_owned(), Some(8443)))
        );
        // Absolute URL with default port normalizes the port away.
        assert_eq!(
            parse_endpoint_authority("https://host.example.net"),
            Some(("host.example.net".to_owned(), None))
        );
        // Bare host, no port.
        assert_eq!(
            parse_endpoint_authority("host.example.net"),
            Some(("host.example.net".to_owned(), None))
        );
        // Surrounding whitespace is tolerated.
        assert_eq!(
            parse_endpoint_authority("  host.example.net:443  "),
            Some(("host.example.net".to_owned(), Some(443)))
        );
        // Malformed inputs yield None so the caller skips routing.
        assert_eq!(parse_endpoint_authority(""), None);
        assert_eq!(parse_endpoint_authority(":443"), None);
        assert_eq!(parse_endpoint_authority("host.example.net:notaport"), None);
    }

    #[test]
    fn apply_layout_endpoint_rewrites_url_and_preserves_host() {
        let mut request = request_to("https://acct.blob.core.windows.net/container/blob");
        apply_layout_endpoint(&mut request, "target.blob.storage.azure.net:443").unwrap();

        assert_eq!(
            request.url().host_str(),
            Some("target.blob.storage.azure.net")
        );
        assert_eq!(request.url().path(), "/container/blob");
        assert_eq!(
            host_header(&request).as_deref(),
            Some("acct.blob.core.windows.net")
        );
    }

    #[test]
    fn apply_layout_endpoint_preserves_original_non_default_port_in_host() {
        let mut request = request_to("https://acct.blob.core.windows.net:10000/container/blob");
        apply_layout_endpoint(&mut request, "target.blob.storage.azure.net:443").unwrap();

        assert_eq!(
            request.url().host_str(),
            Some("target.blob.storage.azure.net")
        );
        assert_eq!(
            host_header(&request).as_deref(),
            Some("acct.blob.core.windows.net:10000")
        );
    }

    #[test]
    fn apply_layout_endpoint_absolute_url_form() {
        let mut request = request_to("https://acct.blob.core.windows.net/container/blob");
        apply_layout_endpoint(&mut request, "https://target.blob.storage.azure.net:8443").unwrap();

        assert_eq!(
            request.url().host_str(),
            Some("target.blob.storage.azure.net")
        );
        assert_eq!(request.url().port(), Some(8443));
        assert_eq!(
            host_header(&request).as_deref(),
            Some("acct.blob.core.windows.net")
        );
    }

    #[test]
    fn apply_layout_endpoint_unparseable_fails_and_leaves_request_untouched() {
        let mut request = request_to("https://acct.blob.core.windows.net/container/blob");
        let result = apply_layout_endpoint(&mut request, "");

        // Malformed endpoint fails the request and does not mutate it.
        assert!(result.is_err());
        assert_eq!(request.url().host_str(), Some("acct.blob.core.windows.net"));
        assert_eq!(host_header(&request), None);
    }
}
