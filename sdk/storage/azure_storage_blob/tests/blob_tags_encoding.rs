// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Verbose, data-driven verification of the `BlobTags` encoding contract.
//!
//! Every test in this file asserts the same end-to-end invariant: **what the customer puts
//! into a `BlobTags` is what `get_tags` returns.** Tag keys and values live in `BlobTags` as
//! raw, unencoded strings; the SDK percent-encodes only when serializing to the `x-ms-tags`
//! header, and serde-xml escapes XML entities only when serializing the `set_tags` body.
//! Both layers are inverted on the read side, so a roundtrip is byte-equal.
//!
//! The matrix below covers the three customer construction styles (`From<HashMap>`, struct
//! literal, response-roundtripped) crossed against the two transports (`set_tags` XML body,
//! `x-ms-tags` header) and against the two data shapes (alphanumeric, special-char). The
//! special-char data set is the reason this file exists: it contains chars (`/`, `+`, `=`,
//! ` `) that are reserved in the `x-ms-tags` header and so MUST be percent-encoded — but
//! must NOT be pre-encoded in `BlobTags` itself, or we'd double-encode on the wire.
//!
//! See `findings.md` next to `Cargo.toml` for the wire data this file is anchored in.

use azure_core::http::{RequestContent, Url};
use azure_core_test::{recorded, Matcher, TestContext, TestMode};
use azure_storage_blob::models::{
    AppendBlobClientCreateOptions, BlobContainerClientFindBlobsByTagsOptions, BlobTag, BlobTags,
    BlockBlobClientCommitBlockListOptions, BlockBlobClientUploadBlobFromUrlOptions,
    BlockBlobClientUploadOptions, BlockLookupList, PageBlobClientCreateOptions,
};
use azure_storage_blob_test::{
    create_test_blob, get_blob_name, get_container_client, StorageAccount,
};
use futures::TryStreamExt;
use std::{collections::HashMap, error::Error, time::Duration};
use tokio::time;

/// Alphanumeric-only data set. Nothing here requires encoding in any transport, so a passing
/// test in this row only proves the basic plumbing works. It's the control case.
fn simple_tags() -> HashMap<String, String> {
    HashMap::from([
        ("env".to_string(), "prod".to_string()),
        ("team".to_string(), "storage".to_string()),
    ])
}

/// Special-char data set. Each entry intentionally contains at least one byte that is reserved
/// in the `x-ms-tags` header and therefore MUST be percent-encoded for that transport:
///
/// - `/` \u2014 unreserved by Azure tag charset, but URL-significant; encoded by the SDK.
/// - `+` \u2014 some URL parsers decode `+` as space; must be encoded.
/// - `=` \u2014 the key/value separator inside the header.
/// - ` ` (space) \u2014 invalid raw in headers.
///
/// `&` is deliberately NOT included because Azure rejects raw `&` in tag values
/// server-side; using it would test server validation, not encoding behavior.
fn special_tags() -> HashMap<String, String> {
    HashMap::from([
        ("path".to_string(), "/a/b".to_string()),
        ("version".to_string(), "1.2".to_string()),
        ("key+name".to_string(), "v=1".to_string()),
        ("with space".to_string(), "x y".to_string()),
    ])
}

/// Builds a `BlobTags` via the documented `From<HashMap>` conversion. Used by the input-style
/// tests below to assert that this conversion is an unencoded passthrough at runtime, mirroring
/// the unit test of the same name in `extensions.rs`.
fn blob_tags_from_hashmap(map: &HashMap<String, String>) -> BlobTags {
    BlobTags::from(map.clone())
}

/// Builds a `BlobTags` via direct struct-literal construction. This is the "manually push in
/// the clutch" path called out in the design discussion: customers who don't have a `HashMap`
/// in hand reach for the model types directly. Values are stored raw, exactly the same as the
/// `From<HashMap>` path \u2014 these tests prove that.
fn blob_tags_from_struct_literal(map: &HashMap<String, String>) -> BlobTags {
    let blob_tag_set = map
        .iter()
        .map(|(k, v)| BlobTag {
            key: Some(k.clone()),
            value: Some(v.clone()),
        })
        .collect();
    BlobTags {
        blob_tag_set: Some(blob_tag_set),
    }
}

// -----------------------------------------------------------------------------
// `set_tags` body path (raw XML values)
// -----------------------------------------------------------------------------

/// Control case: alphanumeric tags built from a `HashMap` round-trip through `set_tags` and
/// `get_tags`. The wire shape (per `findings.md` \u00a71.1\u20131.2) is plain `<Key>`/`<Value>` XML on
/// both request and response; serde-xml-rs handles XML entity escaping transparently and there
/// is no application-level encoding on this path. If this fails, the body path is broken.
#[recorded::test]
async fn set_tags_simple_hashmap_roundtrip(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    recording.set_matcher(Matcher::BodilessMatcher).await?;
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let expected = simple_tags();
    blob_client
        .set_tags(
            RequestContent::try_from(blob_tags_from_hashmap(&expected))?,
            None,
        )
        .await?;

    let response_tags = blob_client.get_tags(None).await?.into_model()?;
    let actual: HashMap<String, String> = response_tags.into();
    assert_eq!(
        expected, actual,
        "alphanumeric tags must round-trip byte-equal through set_tags/get_tags"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// The critical body-path test: special chars (`/`, `+`, `=`, ` `) round-trip through the
/// XML body byte-equal. Because the body path performs **no** percent-encoding (only XML
/// entity escaping at the serde-xml layer), if customers were ever expected to pre-encode
/// they would see `%2F`, `%2B`, etc. in their `get_tags` response. This test will fail
/// loudly in that scenario.
#[recorded::test]
async fn set_tags_special_chars_hashmap_roundtrip(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    recording.set_matcher(Matcher::BodilessMatcher).await?;
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let expected = special_tags();
    blob_client
        .set_tags(
            RequestContent::try_from(blob_tags_from_hashmap(&expected))?,
            None,
        )
        .await?;

    let response_tags = blob_client.get_tags(None).await?.into_model()?;
    let actual: HashMap<String, String> = response_tags.into();
    assert_eq!(
        expected, actual,
        "special-char tags must round-trip byte-equal through the XML body path"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// Same special-char roundtrip as above, but built via struct-literal `BlobTags`. Proves that
/// the struct-literal construction path stores values raw, identically to `From<HashMap>`. If
/// the two paths ever diverge, the customer-visible behavior of `BlobTags` becomes
/// construction-style-dependent, which is a footgun.
#[recorded::test]
async fn set_tags_special_chars_struct_literal_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    recording.set_matcher(Matcher::BodilessMatcher).await?;
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let expected = special_tags();
    blob_client
        .set_tags(
            RequestContent::try_from(blob_tags_from_struct_literal(&expected))?,
            None,
        )
        .await?;

    let response_tags = blob_client.get_tags(None).await?.into_model()?;
    let actual: HashMap<String, String> = response_tags.into();
    assert_eq!(
        expected, actual,
        "struct-literal-built BlobTags must store raw values just like From<HashMap>"
    );

    container_client.delete(None).await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// `x-ms-tags` header path (percent-encoded)
// -----------------------------------------------------------------------------

/// Control case for the header path: alphanumeric tags supplied to `BlockBlobClient::upload`
/// surface byte-equal in `get_tags`. With pure alphanumerics, percent-encoding is a no-op on
/// the wire and this exercises the basic plumbing only.
#[recorded::test]
async fn upload_with_simple_tags_roundtrip(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));

    let expected = simple_tags();
    create_test_blob(
        &blob_client,
        Some(RequestContent::from(b"hello".to_vec())),
        Some(BlockBlobClientUploadOptions {
            tags: Some(blob_tags_from_hashmap(&expected)),
            ..Default::default()
        }),
    )
    .await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "alphanumeric tags supplied to upload must round-trip byte-equal"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// **The critical header-path test.** Special chars supplied to `BlockBlobClient::upload`
/// must be percent-encoded by the SDK exactly once on the way out (so `x-ms-tags` is a valid
/// header) and arrive raw in the subsequent `get_tags` response (so the customer sees what
/// they sent). If `BlobTags` were expected to be pre-encoded by the customer, the
/// percent-encoding would be applied twice and `get_tags` would return `%252F` etc.
#[recorded::test]
async fn upload_with_special_chars_tags_hashmap_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));

    let expected = special_tags();
    create_test_blob(
        &blob_client,
        Some(RequestContent::from(b"hello".to_vec())),
        Some(BlockBlobClientUploadOptions {
            tags: Some(blob_tags_from_hashmap(&expected)),
            ..Default::default()
        }),
    )
    .await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "special-char tags must round-trip byte-equal through the upload header path \
         (no double-encoding, no missing encoding)"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// Struct-literal counterpart to the above: same critical assertion, but the input `BlobTags`
/// is built directly. Confirms the `BlockBlobClient::upload` header-encoding path is agnostic
/// to how `BlobTags` was constructed.
#[recorded::test]
async fn upload_with_special_chars_tags_struct_literal_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));

    let expected = special_tags();
    create_test_blob(
        &blob_client,
        Some(RequestContent::from(b"hello".to_vec())),
        Some(BlockBlobClientUploadOptions {
            tags: Some(blob_tags_from_struct_literal(&expected)),
            ..Default::default()
        }),
    )
    .await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "struct-literal BlobTags must produce the same wire output as From<HashMap>"
    );

    container_client.delete(None).await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// No-double-encoding proof: response \u2192 request roundtrip
// -----------------------------------------------------------------------------

/// **The no-double-encoding test.** Set special-char tags on blob A via the body path, read
/// them back with `get_tags` (which deserializes raw values from XML), then feed that
/// `BlobTags` directly into the upload of blob B (which goes through the header-encoding
/// path). Finally read tags off blob B. The bytes seen on blob B must equal the original
/// input to blob A.
///
/// If `get_tags` were ever to return pre-encoded values (it doesn't \u2014 the XML body carries
/// raw values per `findings.md` \u00a71.2), the upload would percent-encode them a second time and
/// blob B would have `%252F` in place of `/`. This test catches that regression.
#[recorded::test]
async fn response_tags_used_as_upload_input_does_not_double_encode(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    recording.set_matcher(Matcher::BodilessMatcher).await?;
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    // Blob A: write special-char tags via the body path.
    let blob_a = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_a, None, None).await?;
    let expected = special_tags();
    blob_a
        .set_tags(
            RequestContent::try_from(blob_tags_from_hashmap(&expected))?,
            None,
        )
        .await?;

    // Read them back. This BlobTags came from a deserialized XML response and is the exact
    // shape a customer would receive in production.
    let response_tags = blob_a.get_tags(None).await?.into_model()?;

    // Blob B: feed the deserialized BlobTags straight into upload (header path).
    let blob_b = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(
        &blob_b,
        Some(RequestContent::from(b"hello".to_vec())),
        Some(BlockBlobClientUploadOptions {
            tags: Some(response_tags),
            ..Default::default()
        }),
    )
    .await?;

    // Final roundtrip read on blob B must equal the original input to blob A.
    let actual: HashMap<String, String> = blob_b.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "feeding a BlobTags returned by get_tags back into upload must NOT double-encode"
    );

    container_client.delete(None).await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Phase 3 ergonomics: with_tags() on the other tag-bearing operations
// -----------------------------------------------------------------------------

/// Locks the new `with_tags()` builder on `AppendBlobClientCreateOptions`. Customers can now
/// pass a `BlobTags` (raw values) instead of hand-formatting the `x-ms-tags` header string,
/// matching the ergonomics already provided by `BlockBlobClient::upload`.
#[recorded::test]
async fn append_blob_create_with_tags_special_chars_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    let append_blob_client = blob_client.append_blob_client();

    let expected = special_tags();
    let options =
        AppendBlobClientCreateOptions::default().with_tags(&blob_tags_from_hashmap(&expected));
    append_blob_client.create(Some(options)).await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "AppendBlobClient::create with `with_tags` must encode and round-trip byte-equal"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// Locks the new `with_tags()` builder on `PageBlobClientCreateOptions`. Same contract as the
/// append-blob test above.
#[recorded::test]
async fn page_blob_create_with_tags_special_chars_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    let page_blob_client = blob_client.page_blob_client();

    let expected = special_tags();
    let options =
        PageBlobClientCreateOptions::default().with_tags(&blob_tags_from_hashmap(&expected));
    page_blob_client.create(512, Some(options)).await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "PageBlobClient::create with `with_tags` must encode and round-trip byte-equal"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// Locks the new `with_tags()` builder on `BlockBlobClientUploadBlobFromUrlOptions`. Same
/// contract as above. The source blob has no tags; only the destination does, supplied via
/// `with_tags`.
#[recorded::test]
async fn upload_blob_from_url_with_tags_special_chars_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    let source_blob = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(
        &source_blob,
        Some(RequestContent::from(b"source content".to_vec())),
        None,
    )
    .await?;

    let dest_blob = container_client.blob_client(&get_blob_name(recording));
    let dest_block_blob = dest_blob.block_blob_client();
    let expected = special_tags();
    let options = BlockBlobClientUploadBlobFromUrlOptions::default()
        .with_tags(&blob_tags_from_hashmap(&expected));
    dest_block_blob
        .upload_blob_from_url(
            Url::parse(source_blob.url().as_str())?.as_str().into(),
            Some(options),
        )
        .await?;

    let actual: HashMap<String, String> = dest_blob.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "upload_blob_from_url with `with_tags` must encode and round-trip byte-equal"
    );

    container_client.delete(None).await?;
    Ok(())
}

// -----------------------------------------------------------------------------
// Server-side filter sanity
// -----------------------------------------------------------------------------

/// Server-side proof of Observation G in `findings.md`: when a tag value `"/a/b"` is sent
/// via the upload header path (which percent-encodes it), the service decodes the header and
/// stores `/a/b` raw \u2014 confirmed by querying with a filter expression that uses the raw
/// value. If the SDK accidentally double-encoded (e.g., stored `%2Fa%2Fb`), the filter
/// `"path"='/a/b'` would not match.
#[recorded::test]
async fn find_blobs_by_tags_matches_special_char_value(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    recording.set_matcher(Matcher::HeaderlessMatcher).await?;
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    let blob_name = get_blob_name(recording);
    let tags = HashMap::from([("path".to_string(), "/a/b".to_string())]);
    create_test_blob(
        &container_client.blob_client(&blob_name),
        Some(RequestContent::from(b"hello".to_vec())),
        Some(BlockBlobClientUploadOptions {
            tags: Some(blob_tags_from_hashmap(&tags)),
            ..Default::default()
        }),
    )
    .await?;

    // Tag indexing on the service is asynchronous; sleep in live/record mode to give the
    // index time to catch up before querying. Mirrors `test_find_blobs_by_tags`.
    if recording.test_mode() == TestMode::Live || recording.test_mode() == TestMode::Record {
        time::sleep(Duration::from_secs(15)).await;
    }

    let mut pager = container_client
        .find_blobs_by_tags(
            "\"path\"='/a/b'",
            Some(BlobContainerClientFindBlobsByTagsOptions::default()),
        )?
        .into_pages();
    let segment = pager.try_next().await?.unwrap().into_model()?;
    assert!(
        segment
            .blob_items
            .iter()
            .any(|b| b.name.as_deref() == Some(blob_name.as_str())),
        "find_blobs_by_tags must match the raw value '/a/b' on the server side, \
         proving the SDK did not double-encode"
    );

    container_client.delete(None).await?;
    Ok(())
}

/// Locks the new `with_tags()` builder on `BlockBlobClientCommitBlockListOptions`. This is
/// the canonical large-blob upload path (stage_block + commit_block_list), and prior to this
/// builder customers had to hand-format the `x-ms-tags` header. Stage two blocks with raw
/// content, commit them with `with_tags(&special_tags())`, then verify `get_tags` round-trips
/// byte-equal -- proving the header-encoding path on this fourth tag-bearing operation
/// matches the contract verified for the other three.
#[recorded::test]
async fn commit_block_list_with_tags_special_chars_roundtrip(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    let block_blob_client = blob_client.block_blob_client();

    let blocks: &[(&[u8], &[u8])] = &[(b"block-1", b"hello "), (b"block-2", b"world")];
    for (block_id, data) in blocks {
        block_blob_client
            .stage_block(
                block_id,
                u64::try_from(data.len())?,
                RequestContent::from(data.to_vec()),
                None,
            )
            .await?;
    }

    let lookup = BlockLookupList {
        committed: None,
        latest: Some(blocks.iter().map(|(id, _)| id.to_vec()).collect()),
        uncommitted: None,
    };
    let expected = special_tags();
    let options = BlockBlobClientCommitBlockListOptions::default()
        .with_tags(&blob_tags_from_hashmap(&expected));
    block_blob_client
        .commit_block_list(RequestContent::try_from(lookup)?, Some(options))
        .await?;

    let actual: HashMap<String, String> = blob_client.get_tags(None).await?.into_model()?.into();
    assert_eq!(
        expected, actual,
        "commit_block_list with `with_tags` must encode and round-trip byte-equal"
    );

    container_client.delete(None).await?;
    Ok(())
}
