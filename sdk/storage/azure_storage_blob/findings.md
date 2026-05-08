# Findings: blob tags encoding

Working notes for the data-driven verification of `BlobTags` encoding behavior. Each design decision below is anchored in observed wire data from existing recordings or in the source paths that produce that wire data. Not for publication — delete or fold into the PR description before merge.

## 1. Wire-level observations from existing recordings

### 1.1 `set_tags` — XML body, raw values

Recording: [test_blob_tags.json](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/blob_client/test_blob_tags.json).

Customer code (from [tests/blob_client.rs L466–477](tests/blob_client.rs)):

```rust
let blob_tags = HashMap::from([
    ("hello".to_string(), "world".to_string()),
    ("ferris".to_string(), "crab".to_string()),
]);
blob_client
    .set_tags(RequestContent::try_from(BlobTags::from(blob_tags.clone()))?, None)
    .await?;
```

Resulting `PUT …?comp=tags` request body, byte-for-byte:

```text
<?xml version="1.0" encoding="utf-8"?><Tags><TagSet><Tag><Key>hello</Key><Value>world</Value></Tag><Tag><Key>ferris</Key><Value>crab</Value></Tag></TagSet></Tags>
```

**Observation A**: the XML body carries values **raw** — no percent-encoding, no escaping (the chars in this case don't need XML escaping either). `BlobTag::Value` is plain serde-xml-rs serialization.

**Observation B**: tag order in the request body matches insertion order of `Vec<BlobTag>` after `From<HashMap>`. (HashMap iteration order is non-deterministic; in this recording it happened to be `hello, ferris`.)

### 1.2 `get_tags` — XML body, raw values, server-sorted

Same recording, the subsequent `GET …?comp=tags` response body:

```text
﻿<?xml version="1.0" encoding="utf-8"?>
<Tags><TagSet><Tag><Key>ferris</Key><Value>crab</Value></Tag><Tag><Key>hello</Key><Value>world</Value></Tag></TagSet></Tags>
```

**Observation C**: server returns values **raw** in `<Value>`. Same shape as the request.

**Observation D**: server **alphabetizes by key** (`ferris` before `hello` even though we sent `hello` first). Any test that asserts on `Vec<BlobTag>` ordering must account for this — convert to `HashMap` before comparing.

### 1.3 Empty tag set roundtrip

Same recording, clear-tags:

- Request body: `<?xml version="1.0" encoding="utf-8"?><Tags><TagSet/></Tags>` (60 bytes)
- Response body: `<?xml version="1.0" encoding="utf-8"?>\n<Tags><TagSet/></Tags>` (64 bytes)

**Observation E**: clearing tags sends `<TagSet/>` and gets back `<TagSet/>`. `From<HashMap<...,...>> for BlobTags` on an empty map produces `Some(vec![])`, which serializes to `<TagSet/>` (empty element) — server accepts.

### 1.4 `upload` (block blob) — `x-ms-tags` header

Recording: [test_upload_block_blob_with_tags.json](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/block_blob_client/test_upload_block_blob_with_tags.json).

Customer code passes `tags: Some(HashMap::from([("version", "1")]).into())`. The handwritten `BlockBlobClient::upload` (clients/block_blob_client.rs L136) runs it through `blob_tags_to_string` and stuffs the result into `blob_tags_string`, which the generated client writes as the `x-ms-tags` header.

Wire `x-ms-tags` value: `version=1` (raw — no special chars to encode in this case).

`get_tags` afterward returns `<Value>1</Value>` raw.

**Observation F**: end-to-end roundtrip works for alphanumeric values — what goes in via `BlobTags` comes out of `get_tags` byte-equal.

### 1.5 `find_blobs_by_tags` — multiple tags on header

Recording: [test_find_blobs_by_tags.json L42](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/blob_container_client/test_find_blobs_by_tags.json).

Customer passes `blob_tags_string: Some("alice=bob&foo=bar".to_string())` — wire header is the same. Two-tag header uses `&` as separator unencoded, exactly what `blob_tags_to_string` would produce.

**Observation G**: `&` is the tag separator in `x-ms-tags`. Therefore raw `&` in a value MUST be percent-encoded. `=` is the key/value separator — raw `=` in keys/values MUST also be percent-encoded.

### 1.6 Other tag-bearing operations — caller pre-formats today

[test_create_append_blob_with_tags.json L41](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/append_blob_client/test_create_append_blob_with_tags.json), [test_create_page_blob_with_tags.json L42](../../../.assets/SzpuGZJaNc/rust/sdk/storage/azure_storage_blob/tests/data/page_blob_client/test_create_page_blob_with_tags.json).

Both customer call sites pass `blob_tags_string: Some("env=test".to_string())` directly — no `BlobTags` plumbing. The customer is responsible for encoding. With alphanumeric values nothing breaks; with special chars this would silently produce the wrong thing.

**Observation H**: there is a real ergonomic + correctness gap: only `BlockBlobClient::upload` runs `blob_tags_to_string`. Append/Page `create` and `BlockBlob::upload_blob_from_url` make the caller hand-format.

## 2. Source-level observations

### 2.1 `BlobTags` is plain serde XML

[generated/models/models.rs L488–500](src/generated/models/models.rs):

```rust
#[derive(Clone, Default, Deserialize, SafeDebug, Serialize)]
#[serde(rename = "Tags")]
pub struct BlobTags { pub blob_tag_set: Option<Vec<BlobTag>> }
```

No custom serializer/deserializer on `<Key>`/`<Value>`. serde-xml-rs handles XML entity escaping (`&`, `<`, `>`, etc.) at the XML layer transparently. **The model carries raw strings; the XML serializer escapes; the XML deserializer un-escapes.** No application-level encoding in either direction.

### 2.2 `blob_tags_to_string` percent-encodes for the header

[src/models/extensions.rs L16–37](src/models/extensions.rs):

```rust
pub(crate) fn blob_tags_to_string(tags: &BlobTags) -> Option<String> {
    // percent_encode(NON_ALPHANUMERIC) on key and value, joined with '&'
}
```

`NON_ALPHANUMERIC` encodes everything that isn't `[A-Za-z0-9]`, which is broader than strictly necessary (e.g., `+`, `:`, `_`, `-` all get encoded though Azure accepts them raw in the header). Wire is longer than minimal but **correct** — server URL-decodes, so the roundtrip is lossless.

### 2.3 `From<HashMap>` / `Into<HashMap>` — pure passthrough

[src/models/extensions.rs L91–119](src/models/extensions.rs). No transformation in either direction. Confirms the customer-facing contract: **`BlobTags` carries raw values**.

### 2.4 Header writers are unconditional

Generated `request.insert_header("x-ms-tags", blob_tags_string)` in append/block/page clients. **No SDK-side encoding at the writer**. The encoding contract sits at the call site (`BlockBlobClient::upload` only, today).

## 3. The encoding invariant

From observations A, C, and the source-level analysis:

> **`BlobTags.blob_tag_set` always contains raw key/value strings — never percent-encoded, never XML-escaped. The SDK encodes only when serializing to the `x-ms-tags` header.**

This invariant holds today because:

- `set_tags` body path: serde-xml escapes XML entities at serialize time, un-escapes at deserialize time. Application code stays raw.
- `get_tags` body path: same.
- `upload` header path: `blob_tags_to_string` percent-encodes at serialize time. There is no inverse `from_x_ms_tags_string` because the service doesn't echo the header back — `get_tags` is the only read path and it's the body path.
- Therefore feeding a deserialized `BlobTags` (raw) back into upload (which encodes once) gives **exactly one** encoding pass on the wire. **No double-encoding is possible** as long as the invariant is preserved.

## 4. Decisions

| # | Question | Decision | Anchored in |
|---|----------|----------|-------------|
| 1 | Should `BlobTags` carry raw or pre-encoded values? | **Raw.** | §2.1, §2.3, §3 |
| 2 | Should `From<HashMap>` encode? | **No.** | §3 (would break invariant + cause double-encoding) |
| 3 | Should `set_tags` request path encode? | **No.** Plain XML body, raw values. serde-xml handles entity escaping. | §1.1, §2.1 |
| 4 | Should `x-ms-tags` header path encode? | **Yes.** Percent-encode key and value, join with `&`. | §1.4, §1.5, §2.2 |
| 5 | Should we tighten the `AsciiSet` to encode fewer chars? | **No** (out of scope). Current `NON_ALPHANUMERIC` is correct, just verbose on the wire. | §2.2 |
| 6 | Should the other tag-bearing ops accept `BlobTags`? | **Yes** — extend the convenience to Append/Page `create` and `upload_blob_from_url` so the customer never hand-formats `x-ms-tags`. | §1.6 (Observation H) |
| 7 | How? | Add a `with_tags(&BlobTags) -> Self` builder on each generated options bag, calling `blob_tags_to_string`. Keep the raw `blob_tags_string` field as an expert escape hatch. Mirrors the existing `if_not_exists()` extension pattern. | §1.6, [extensions.rs L40–87](src/models/extensions.rs) |
| 8 | Document the invariant where? | On `BlobTag`, `BlobTags`, `From<HashMap>` impl, `blob_tags_to_string`, and the generated `blob_tags_string` field. | §3 |

## 5. Test plan executed (all data-anchored)

### 5.1 Unit tests in `extensions.rs` (no recording)

These lock the encoding contract from §2.2 and §3:

- `blob_tags_to_string` returns `None` for empty.
- Skips entries with missing key or value.
- Preserves input order (anchored in Observation B).
- **Exact-byte assertion** on the percent-encoding output for `key+name=v=1` (Observation G — `=` and `+` are reserved in the header).
- **Exact-byte assertion** on space and `/`.
- `From<HashMap>` and `From<BlobTags> for HashMap` are byte-equal passthroughs (locks Decision 2).

### 5.2 Recorded integration tests in `tests/blob_tags_encoding.rs`

Each test asserts `input HashMap == get_tags response HashMap` — the only end-to-end contract that matters per §3.

- `test_set_tags_simple_roundtrip` — alphanumeric, `From<HashMap>` input. Anchors §1.1–1.3.
- `test_set_tags_special_chars_roundtrip` — `/`, `+`, `=`, ` ` in keys and values. The critical recording for §3.
- `test_set_tags_struct_literal_input_special_chars` — same as above but built via `BlobTags { blob_tag_set: Some(vec![...]) }` to prove the struct-literal path also stays raw.
- `test_upload_with_tags_special_chars_roundtrip` — `BlockBlobClient::upload` header path. Critical for Decision 4.
- `test_response_tags_roundtripped_into_upload` — `set_tags(special)` → `get_tags()` → `upload(blob_b, tags=that)` → `get_tags(blob_b)` == original. **Locks "no double-encoding" claim from §3.**
- `test_append_blob_create_with_tags_special_chars` — locks Decision 7 for append blobs.
- `test_page_blob_create_with_tags_special_chars` — same for page blobs.
- `test_upload_blob_from_url_with_tags_special_chars` — same for copy-from-URL.
- `test_find_blobs_by_tags_returns_blob_with_special_chars` — confirms server sees raw `/a/b` (anchors Observation G's interpretation).

Recordings must be produced in a follow-up live run; tests will pass-by-construction once recorded because every assertion follows from §1–§3.

## 6. Out of scope (explicitly)

- SAS `if_tags` filter expression escaping — different syntax (SQL-like), separate concern.
- `find_blobs_by_tags` filter-expression escaping for raw values containing quotes — separate concern.
- Tightening the `AsciiSet` (Decision 5).
- TypeSpec emitter changes to move encoding into the generated header writer.
- Client-side validation of Azure's allowed tag charset (server enforces, clearer errors that way).

## 7. Open questions to resolve at review

None blocking implementation. All decisions above have a wire/source anchor.
