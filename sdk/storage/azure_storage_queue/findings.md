# Findings: queue encoding surfaces (companion to blob tags analysis)

Working notes paralleling `sdk/storage/azure_storage_blob/findings.md`, applied to `azure_storage_queue`. Same data-driven approach: identify every place customer strings cross a transport boundary, check what the SDK does, decide what (if anything) to change.

**Bottom line: nothing to fix in this crate as part of this PR.** Queues do not have the two-transport ambiguity that motivated the blob tags work. Each encoding-relevant surface uses exactly one transport with exactly one encoding regime, and existing tests already prove the roundtrip. Details below.

## 0. TL;DR

| Surface | Transport | What the SDK does | Already verified by | Action |
|---|---|---|---|---|
| `QueueMessage::message_text` (send) and `ReceivedMessage::message_text` (receive/peek) | XML body inside `<MessageText>` | serde-xml escapes XML entities (`&`, `<`, `>`) on send; un-escapes on receive. **No base64**, no percent-encoding. | `test_send_message_unicode`, `test_update_message_unicode`, the `\u{0001}` control-char rejection test, whitespace test, near-64KB test in `tests/queue_client.rs`. | None. |
| Queue metadata (`metadata: HashMap<String, String>` on `QueueClientCreateOptions` / `set_metadata`) | `x-ms-meta-{k}: v` headers, one per entry | Pure passthrough -- generated client emits `format!("x-ms-meta-{k}")` and writes the value as-is. | `test_list_queues_include_metadata` (queue_service_client.rs). | None this PR -- same posture as blob metadata, which is also out of scope. |

## 1. Why there's no transport-split here

The blob tags fix was needed because **the same data type (`BlobTags`) flows through two transports** (XML body for `set_tags`, `x-ms-tags` header for `upload`/etc.) with different encoding rules. Customers could be tempted to pre-encode and double-encode.

Queues do not have that shape:

- **`message_text` is body-only.** There is no `x-ms-message` header equivalent. So there's only the XML-body encoding path, which serde-xml handles transparently.
- **Queue metadata is header-only.** There is no body equivalent. So there's only the header path, which is plain ASCII passthrough by service contract.

Neither surface admits a "construction-style mismatch" or "double-encoding" failure mode equivalent to `BlobTags`.

## 2. Source-level confirmation

### 2.1 `QueueMessage` is plain serde XML

[generated/models/models.rs L282-291](src/generated/models/models.rs):

```rust
#[derive(Clone, Default, Deserialize, SafeDebug, Serialize)]
#[serde(rename = "QueueMessage")]
pub struct QueueMessage {
    #[serde(rename = "MessageText", skip_serializing_if = "Option::is_none")]
    pub message_text: Option<String>,
}
```

No custom serializer/deserializer, no base64 hook. Identical pattern to `BlobTag`/`BlobTags` -- the model carries raw strings; serde-xml escapes XML entities at the layer below; deserialize un-escapes. The same invariant from blob tags applies: **`message_text` always holds raw values**.

### 2.2 No SDK-side encoding for queue metadata

[generated/clients/queue_client.rs L108-112](src/generated/clients/queue_client.rs):

```rust
if let Some(metadata) = options.metadata.as_ref() {
    for (k, v) in metadata {
        request.insert_header(format!("x-ms-meta-{k}"), v);
    }
}
```

Direct passthrough. If the customer passes a non-ASCII value, the underlying HTTP layer will reject (or the service will). This is identical to how blob metadata works today -- a known shared limitation across Storage services, not a queue-specific bug.

## 3. Cross-SDK note (informational, not actionable here)

Python, .NET, and Java queue clients ship an optional `MessageEncodePolicy` (typically base64) on the queue-client level so that customers can store binary or XML-illegal payloads transparently. The Rust SDK does **not** ship that abstraction; customers who need binary safety must base64 the bytes themselves before assigning to `message_text`.

This is a separate feature-parity question from the blob tags encoding correctness work. The current Rust behavior is **correct** -- raw text in, raw text out, service rejects XML-illegal control chars (per the existing `\u{0001}` test) -- but it is **less ergonomic** than the other-language SDKs for binary use cases. Worth a follow-up issue if/when there's customer demand. Out of scope here.

## 4. Conclusion

The blob tags encoding contract documented in `sdk/storage/azure_storage_blob/findings.md` does NOT need a queue-side counterpart. No code or test changes proposed for `azure_storage_queue` in this PR.
