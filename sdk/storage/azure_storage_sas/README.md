<!-- cspell:ignore udk -->
# Azure Storage SAS client library for Rust

Generate Shared Access Signature (SAS) tokens for Azure Storage services.

This crate is a pure CPU library: it performs no HTTP I/O. It signs SAS tokens
using a [User Delegation Key][udk] previously obtained from a Storage service
client (for example, `BlobServiceClient::get_user_delegation_key`).

> This release supports **user delegation SAS** only. Account-key SAS and
> account SAS are not yet supported.

## Getting started

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
azure_storage_sas = "1.0.0"
```

## Examples

Sign a SAS for a single blob using a user delegation key:

```rust
use azure_storage_sas::{
    BlobSasBuilder, BlobSasPermissions, SasProtocol, UserDelegationKey,
};
use time::{Duration, OffsetDateTime};

# fn main() -> azure_core::Result<()> {
let now = OffsetDateTime::now_utc();

// In real code, obtain this by calling
// `BlobServiceClient::get_user_delegation_key`.
let udk = UserDelegationKey::new(
    "00000000-0000-0000-0000-000000000000",
    "00000000-0000-0000-0000-000000000000",
    now,
    now + Duration::hours(1),
    "b",
    "2026-04-06",
    "dGVzdC1rZXk=", // base64-encoded secret
);

let sas = BlobSasBuilder::new(
    "my-container".to_string(),
    "my-blob.txt".to_string(),
    now + Duration::hours(1),
    BlobSasPermissions::read_only(),
)
.start(now)
.protocol(SasProtocol::Https)
.with_key(udk)
.sign("myaccount")?;

let url = sas.to_url("https://myaccount.blob.core.windows.net")?;
assert!(url.contains("sig="));
# Ok(()) }
```

## Contributing

See the [contributing guide][contrib] in the repository root.

[udk]: https://learn.microsoft.com/rest/api/storageservices/create-user-delegation-sas
[contrib]: https://github.com/Azure/azure-sdk-for-rust/blob/main/CONTRIBUTING.md
