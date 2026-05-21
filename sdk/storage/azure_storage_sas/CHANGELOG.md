# Release History

## 1.0.0 (Unreleased)

### Features Added

- Initial release of the `azure_storage_sas` crate with user-delegation SAS support for Azure Blob storage (`BlobSasBuilder`, `BlobContainerSasBuilder`).
- Snapshot and blob version SAS support via `BlobSasBuilder::snapshot` and `BlobSasBuilder::blob_version`.
- Delegated user identity support via `delegated_user_tenant_id` (`skdtid`) and `delegated_user_object_id` (`sduoid`) setters.
- `Display` and `FromStr` for `BlobSasPermissions`, `BlobContainerSasPermissions`, and `SasIpRange`, enabling round-trip with the canonical Azure string formats.
- `BlobSasPermissions::all`, `BlobSasPermissions::read_only`, `BlobContainerSasPermissions::all`, and `BlobContainerSasPermissions::read_only` convenience constructors.
- Validation in `sign()` rejecting empty permissions and an expiry that is not after `start`.
