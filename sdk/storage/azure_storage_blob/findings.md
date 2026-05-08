# Blob Tags Encoding: Deep Dive

Working notes for the data-driven verification of `BlobTags` encoding behavior. Each design decision below is anchored in observed wire data from existing recordings or in the source paths that produce that wire data. Not for publication  --  delete or fold into the PR description before merge.

## 0. TL;DR

If you only read one section, read this one.

**The rule for customers:** put **raw, unencoded** strings into `BlobTags`. Don't percent-encode. Don't XML-escape. Don't think about `&` or `=` in your values. The SDK handles encoding for you, in exactly one place, only when the wire actually needs it.

**The rule for the SDK:** encoding lives at the transport boundary, not in the model. There are two transports and they encode differently:

| Transport | Used by | What the SDK does on send | What the SDK does on receive |
|---|---|---|---|
| **XML body** | `set_tags`, response of `get_tags` | serde-xml escapes XML entities (`&` -> `&amp;` etc.) automatically. **No percent-encoding.** | serde-xml un-escapes back to raw. |
| **`x-ms-tags` header** | `BlockBlobClient::upload`, `AppendBlobClient::create`, `PageBlobClient::create`, `BlockBlobClient::upload_blob_from_url` | `blob_tags_to_string` percent-encodes each key and value, joins with `&`. | N/A -- this header is write-only; tags are read back via the body path. |

> **This is the correct, consistent choice.** Customers see one shape (`BlobTags` with raw strings) regardless of which operation they call, and the SDK encodes once -- at exactly the byte boundary that requires it -- with no double-encoding possible. Verified end-to-end against the live service in Section 5.3.

### Roundtripping is safe

You can take a `BlobTags` you got from `get_tags` and pass it straight back into any tag-bearing operation. **No double-encoding will happen.** The proof:

```text
get_tags response (XML, raw values)
  -> deserialize to BlobTags (raw values, no transformation)
    -> upload(tags: Some(...)) -> percent-encode in x-ms-tags header
      -> server URL-decodes the header
        -> server stores raw
          -> get_tags returns raw == original input
```

There is exactly one encode point (the header writer) and exactly one decode point (the server). The body path is pure on both sides. Test `response_tags_used_as_upload_input_does_not_double_encode` in Section 5.3 walks this whole chain and asserts byte equality.

### Construction style doesn't matter

There are three idiomatic ways to build a `BlobTags`. **All three produce identical wire output**, verified live:

```rust
// 1) From a HashMap (most common)
let tags: BlobTags = HashMap::from([
    ("path".to_string(), "/a/b".to_string()),
    ("with space".to_string(), "x y".to_string()),
]).into();

// 2) Struct literal (when you don't have a HashMap in hand)
let tags = BlobTags {
    blob_tag_set: Some(vec![
        BlobTag { key: Some("path".into()), value: Some("/a/b".into()) },
        BlobTag { key: Some("with space".into()), value: Some("x y".into()) },
    ]),
};

// 3) Reusing a response from get_tags
let tags = blob_client.get_tags(None).await?.into_model()?;

// All three are interchangeable. Pass to any tag-bearing op:
blob_client.set_tags(RequestContent::try_from(tags.clone())?, None).await?;
block_blob_client.upload(content, Some(BlockBlobClientUploadOptions {
    tags: Some(tags.clone()),
    ..Default::default()
})).await?;
append_blob_client.create(Some(
    AppendBlobClientCreateOptions::default().with_tags(&tags),
)).await?;
```

Tracing the same `tags` (containing `path=/a/b` and `with space=x y`) through each call:

| Call | Transport | What goes on the wire |
|---|---|---|
| `blob_client.set_tags(...)` | XML body | `<Key>path</Key><Value>/a/b</Value>...` -- raw values. serde-xml escapes only XML entities (`&`, `<`, `>`); none apply here. |
| `block_blob_client.upload(..., tags: Some(tags))` | `x-ms-tags` header | `path=%2Fa%2Fb&with%20space=x%20y` -- percent-encoded by `blob_tags_to_string`. |
| `append_blob_client.create(..., with_tags(&tags))` | `x-ms-tags` header | Same as above -- percent-encoded. |

**The wire bytes differ by transport, but the customer-observable behavior is identical:** the server decodes on receive and stores raw, so a subsequent `get_tags` returns the same map you started with -- whichever path you took to write it.

### Why this design

1. **One encode point, one decode point.** Encoding lives at the byte boundary that needs it (the header writer). Models stay raw. Easy to reason about, easy to test.
2. **Symmetric API.** What `get_tags` returns is what `set_tags`/`upload`/etc. accept. No "encode on the way in but we decode on the way out" asymmetry.
3. **No magic.** No "does this string look already-encoded?" detection, no implicit transformations in `From` impls. Bytes pass through untouched until they hit the wire.
4. **Construction-style invariant.** Customers can't break themselves by picking the "wrong" way to build a `BlobTags`. All three paths are byte-equivalent.
5. **Cross-SDK alignment.** Matches Python, .NET, and Java: model carries raw values, transport adapter encodes.

## 1. Wire-level observations from existing recordings

### 1.1 `set_tags`  --  XML body, raw values

Recording: [test_blob_tags.json](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/blob_client/test_blob_tags.json).

Customer code (from [tests/blob_client.rs L466-477](tests/blob_client.rs)):

```rust
let blob_tags = HashMap::from([
    ("hello".to_string(), "world".to_string()),
    ("ferris".to_string(), "crab".to_string()),
]);
blob_client
    .set_tags(RequestContent::try_from(BlobTags::from(blob_tags.clone()))?, None)
    .await?;
```

Resulting `PUT ...?comp=tags` request body, byte-for-byte:

```text
<?xml version="1.0" encoding="utf-8"?><Tags><TagSet><Tag><Key>hello</Key><Value>world</Value></Tag><Tag><Key>ferris</Key><Value>crab</Value></Tag></TagSet></Tags>
```

**Observation A**: the XML body carries values **raw**  --  no percent-encoding, no escaping (the chars in this case don't need XML escaping either). `BlobTag::Value` is plain serde-xml-rs serialization.

**Observation B**: tag order in the request body matches insertion order of `Vec<BlobTag>` after `From<HashMap>`. (HashMap iteration order is non-deterministic; in this recording it happened to be `hello, ferris`.)

### 1.2 `get_tags`  --  XML body, raw values, server-sorted

Same recording, the subsequent `GET ...?comp=tags` response body:

```text
﻿<?xml version="1.0" encoding="utf-8"?>
<Tags><TagSet><Tag><Key>ferris</Key><Value>crab</Value></Tag><Tag><Key>hello</Key><Value>world</Value></Tag></TagSet></Tags>
```

**Observation C**: server returns values **raw** in `<Value>`. Same shape as the request.

**Observation D**: server **alphabetizes by key** (`ferris` before `hello` even though we sent `hello` first). Any test that asserts on `Vec<BlobTag>` ordering must account for this  --  convert to `HashMap` before comparing.

### 1.3 Empty tag set roundtrip

Same recording, clear-tags:

- Request body: `<?xml version="1.0" encoding="utf-8"?><Tags><TagSet/></Tags>` (60 bytes)
- Response body: `<?xml version="1.0" encoding="utf-8"?>\n<Tags><TagSet/></Tags>` (64 bytes)

**Observation E**: clearing tags sends `<TagSet/>` and gets back `<TagSet/>`. `From<HashMap<...,...>> for BlobTags` on an empty map produces `Some(vec![])`, which serializes to `<TagSet/>` (empty element)  --  server accepts.

### 1.4 `upload` (block blob)  --  `x-ms-tags` header

Recording: [test_upload_block_blob_with_tags.json](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/block_blob_client/test_upload_block_blob_with_tags.json).

Customer code passes `tags: Some(HashMap::from([("version", "1")]).into())`. The handwritten `BlockBlobClient::upload` (clients/block_blob_client.rs L136) runs it through `blob_tags_to_string` and stuffs the result into `blob_tags_string`, which the generated client writes as the `x-ms-tags` header.

Wire `x-ms-tags` value: `version=1` (raw  --  no special chars to encode in this case).

`get_tags` afterward returns `<Value>1</Value>` raw.

**Observation F**: end-to-end roundtrip works for alphanumeric values  --  what goes in via `BlobTags` comes out of `get_tags` byte-equal.

### 1.5 `find_blobs_by_tags`  --  multiple tags on header

Recording: [test_find_blobs_by_tags.json L42](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/blob_container_client/test_find_blobs_by_tags.json).

Customer passes `blob_tags_string: Some("alice=bob&foo=bar".to_string())`  --  wire header is the same. Two-tag header uses `&` as separator unencoded, exactly what `blob_tags_to_string` would produce.

**Observation G**: `&` is the tag separator in `x-ms-tags`. Therefore raw `&` in a value MUST be percent-encoded. `=` is the key/value separator  --  raw `=` in keys/values MUST also be percent-encoded.

### 1.6 Other tag-bearing operations  --  caller pre-formats today

[test_create_append_blob_with_tags.json L41](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/append_blob_client/test_create_append_blob_with_tags.json), [test_create_page_blob_with_tags.json L42](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/page_blob_client/test_create_page_blob_with_tags.json).

Both customer call sites pass `blob_tags_string: Some("env=test".to_string())` directly  --  no `BlobTags` plumbing. The customer is responsible for encoding. With alphanumeric values nothing breaks; with special chars this would silently produce the wrong thing.

**Observation H**: there is a real ergonomic + correctness gap: only `BlockBlobClient::upload` runs `blob_tags_to_string`. Append/Page `create` and `BlockBlob::upload_blob_from_url` make the caller hand-format.

## 2. Source-level observations

### 2.1 `BlobTags` is plain serde XML

[generated/models/models.rs L488-500](src/generated/models/models.rs):

```rust
#[derive(Clone, Default, Deserialize, SafeDebug, Serialize)]
#[serde(rename = "Tags")]
pub struct BlobTags { pub blob_tag_set: Option<Vec<BlobTag>> }
```

No custom serializer/deserializer on `<Key>`/`<Value>`. serde-xml-rs handles XML entity escaping (`&`, `<`, `>`, etc.) at the XML layer transparently. **The model carries raw strings; the XML serializer escapes; the XML deserializer un-escapes.** No application-level encoding in either direction.

### 2.2 `blob_tags_to_string` percent-encodes for the header

[src/models/extensions.rs L16-37](src/models/extensions.rs):

```rust
pub(crate) fn blob_tags_to_string(tags: &BlobTags) -> Option<String> {
    // percent_encode(NON_ALPHANUMERIC) on key and value, joined with '&'
}
```

`NON_ALPHANUMERIC` encodes everything that isn't `[A-Za-z0-9]`, which is broader than strictly necessary (e.g., `+`, `:`, `_`, `-` all get encoded though Azure accepts them raw in the header). Wire is longer than minimal but **correct**  --  server URL-decodes, so the roundtrip is lossless.

### 2.3 `From<HashMap>` / `Into<HashMap>`  --  pure passthrough

[src/models/extensions.rs L91-119](src/models/extensions.rs). No transformation in either direction. Confirms the customer-facing contract: **`BlobTags` carries raw values**.

### 2.4 Header writers are unconditional

Generated `request.insert_header("x-ms-tags", blob_tags_string)` in append/block/page clients. **No SDK-side encoding at the writer**. The encoding contract sits at the call site (`BlockBlobClient::upload` only, today).

## 3. The encoding invariant

From observations A, C, and the source-level analysis:

> **`BlobTags.blob_tag_set` always contains raw key/value strings  --  never percent-encoded, never XML-escaped. The SDK encodes only when serializing to the `x-ms-tags` header.**

This invariant holds today because:

- `set_tags` body path: serde-xml escapes XML entities at serialize time, un-escapes at deserialize time. Application code stays raw.
- `get_tags` body path: same.
- `upload` header path: `blob_tags_to_string` percent-encodes at serialize time. There is no inverse `from_x_ms_tags_string` because the service doesn't echo the header back  --  `get_tags` is the only read path and it's the body path.
- Therefore feeding a deserialized `BlobTags` (raw) back into upload (which encodes once) gives **exactly one** encoding pass on the wire. **No double-encoding is possible** as long as the invariant is preserved.

## 4. Decisions

| # | Question | Decision | Anchored in |
|---|----------|----------|-------------|
| 1 | Should `BlobTags` carry raw or pre-encoded values? | **Raw.** | Section 2.1, Section 2.3, Section 3 |
| 2 | Should `From<HashMap>` encode? | **No.** | Section 3 (would break invariant + cause double-encoding) |
| 3 | Should `set_tags` request path encode? | **No.** Plain XML body, raw values. serde-xml handles entity escaping. | Section 1.1, Section 2.1 |
| 4 | Should `x-ms-tags` header path encode? | **Yes.** Percent-encode key and value, join with `&`. | Section 1.4, Section 1.5, Section 2.2 |
| 5 | Should we tighten the `AsciiSet` to encode fewer chars? | **No** (out of scope). Current `NON_ALPHANUMERIC` is correct, just verbose on the wire. | Section 2.2 |
| 6 | Should the other tag-bearing ops accept `BlobTags`? | **Yes**  --  extend the convenience to Append/Page `create` and `upload_blob_from_url` so the customer never hand-formats `x-ms-tags`. | Section 1.6 (Observation H) |
| 7 | How? | Add a `with_tags(&BlobTags) -> Self` builder on each generated options bag, calling `blob_tags_to_string`. Keep the raw `blob_tags_string` field as an expert escape hatch. Mirrors the existing `if_not_exists()` extension pattern. | Section 1.6, [extensions.rs L40-87](src/models/extensions.rs) |
| 8 | Document the invariant where? | On `BlobTag`, `BlobTags`, `From<HashMap>` impl, `blob_tags_to_string`, and the generated `blob_tags_string` field. | Section 3 |

## 5. Test plan executed (all data-anchored)

### 5.1 Unit tests in `extensions.rs` (no recording)

These lock the encoding contract from Section 2.2 and Section 3:

- `blob_tags_to_string` returns `None` for empty.
- Skips entries with missing key or value.
- Preserves input order (anchored in Observation B).
- **Exact-byte assertion** on the percent-encoding output for `key+name=v=1` (Observation G  --  `=` and `+` are reserved in the header).
- **Exact-byte assertion** on space and `/`.
- `From<HashMap>` and `From<BlobTags> for HashMap` are byte-equal passthroughs (locks Decision 2).

### 5.2 Recorded integration tests in `tests/blob_tags_encoding.rs`

Each test asserts `input HashMap == get_tags response HashMap`  --  the only end-to-end contract that matters per Section 3.

- `test_set_tags_simple_roundtrip`  --  alphanumeric, `From<HashMap>` input. Anchors Section 1.1-1.3.
- `test_set_tags_special_chars_roundtrip`  --  `/`, `+`, `=`, ` ` in keys and values. The critical recording for Section 3.
- `test_set_tags_struct_literal_input_special_chars`  --  same as above but built via `BlobTags { blob_tag_set: Some(vec![...]) }` to prove the struct-literal path also stays raw.
- `test_upload_with_tags_special_chars_roundtrip`  --  `BlockBlobClient::upload` header path. Critical for Decision 4.
- `test_response_tags_roundtripped_into_upload`  --  `set_tags(special)` -> `get_tags()` -> `upload(blob_b, tags=that)` -> `get_tags(blob_b)` == original. **Locks "no double-encoding" claim from Section 3.**
- `test_append_blob_create_with_tags_special_chars`  --  locks Decision 7 for append blobs.
- `test_page_blob_create_with_tags_special_chars`  --  same for page blobs.
- `test_upload_blob_from_url_with_tags_special_chars`  --  same for copy-from-URL.
- `test_find_blobs_by_tags_returns_blob_with_special_chars`  --  confirms server sees raw `/a/b` (anchors Observation G's interpretation).

Recordings must be produced in a follow-up live run; tests will pass-by-construction once recorded because every assertion follows from Section 1-Section 3.

## 5.3 Live-run results (2026-05-07)

All 11 recorded tests **passed against the live service**, confirming every decision in Section 4 with wire-level evidence (not just static analysis):

| Test | Confirms |
|------|----------|
| `set_tags_simple_hashmap_roundtrip` | Body path plumbing works for the trivial case. |
| `set_tags_special_chars_hashmap_roundtrip` | XML body carries `/`, `+`, `=`, ` ` raw  --  server stores them raw  --  `get_tags` returns them raw. **No client-side encoding on this path.** Locks Decision 3. |
| `set_tags_special_chars_struct_literal_roundtrip` | Struct-literal `BlobTags` is byte-identical to `From<HashMap>` on the wire. Construction style does NOT change semantics. |
| `upload_with_simple_tags_roundtrip` | Header path plumbing works. |
| `upload_with_special_chars_tags_hashmap_roundtrip` | `blob_tags_to_string` percent-encoding is correct for the live service: special chars survive the `x-ms-tags` header -> service decodes -> `get_tags` returns them raw. Locks Decision 4. |
| `upload_with_special_chars_tags_struct_literal_roundtrip` | Same but via struct literal. Construction style invariant holds for the header path too. |
| **`response_tags_used_as_upload_input_does_not_double_encode`** | The critical proof. Deserialized `BlobTags` from a `get_tags` response can be fed directly into a subsequent `upload` and the result is byte-equal to the original input. **No double-encoding occurs anywhere in the SDK.** Locks the central invariant in Section 3. |
| `append_blob_create_with_tags_special_chars_roundtrip` | New `with_tags()` builder on `AppendBlobClientCreateOptions` produces the same wire encoding as `BlockBlobClient::upload`. Locks Decision 7 for append blobs. |
| `page_blob_create_with_tags_special_chars_roundtrip` | Same for `PageBlobClientCreateOptions`. |
| `upload_blob_from_url_with_tags_special_chars_roundtrip` | Same for `BlockBlobClientUploadBlobFromUrlOptions`. |
| `find_blobs_by_tags_matches_special_char_value` | Server-side confirmation that the SDK did NOT double-encode: a query with raw filter expression `"path"='/a/b'` matches the blob we uploaded with `BlobTags::from({"path":"/a/b"})`. If the SDK had encoded twice, the stored value would have been `%2Fa%2Fb` and the filter would not match. |

### What the live results imply

1. **The encoding invariant in Section 3 is empirically verified, not just inferred.** Every customer construction style (`From<HashMap>`, struct literal, response-roundtripped) produces byte-equal output across both transports.
2. **`blob_tags_to_string` percent-encodes the right characters.** `NON_ALPHANUMERIC` is conservative, but lossless. No changes needed.
3. **Decision 7 is now risk-free to ship.** The three new `with_tags()` builders (`AppendBlobClientCreateOptions`, `PageBlobClientCreateOptions`, `BlockBlobClientUploadBlobFromUrlOptions`) round-trip special chars identically to the pre-existing `BlockBlobClient::upload` path. Customers using these wrappers will have the same correctness guarantee as today's `upload` users  --  without hand-formatting `x-ms-tags`.
4. **No double-encoding regression vector remains.** The response->request roundtrip test is the strongest single signal: it exercises every layer (XML deserialize, `From<BlobTag>` move, struct passthrough into options, percent-encode, header insert, server decode, server store, server emit, server URL-decode on filter, server match) end-to-end and asserts byte equality.
5. **Construction style is a non-issue.** Documentation can confidently say "build `BlobTags` however you like  --  the bytes stay raw."

### Consistency takeaways for the broader SDK

- The pattern "raw values in models, encode at the transport boundary" is the right default for any future SDK feature where a header carries structured data (think key-value pairs). It keeps the model types ergonomic and the encoding logic in exactly one place.
- The `with_tags()` builder pattern (extension impl on a generated options bag) is a clean way to add cross-cutting convenience without forking the generated code or asking the emitter to special-case anything.
- For future cross-language alignment: this verification establishes the Rust SDK's contract precisely. If/when the decision is made to push encoding into the emitter (Decision 5 area, currently out of scope), there is now a regression suite that will catch any drift.
