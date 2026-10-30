// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

//! Local-only smoke test; no test proxy or recordings.
//!
//! Start the locally patched Azurite with `--oauth basic`; no `az login` is needed.
//! Run `cargo test -p azure_storage_blob --test azurite --all-features -- --ignored --nocapture`.
//! For HTTPS, start Azurite with `--oauth basic --cert <certificate> --key <key>`
//! using a trusted certificate for `127.0.0.1`, then run only
//! `cargo test -p azure_storage_blob --test azurite --all-features upload_foo_get_properties_https -- --ignored --nocapture --exact`.
//! The HTTPS test does not require the local HTTP authentication patches.
//! The test leaves its uniquely named container and blob available for inspection.
//! Uses a static, unsigned JWT valid from 2020 through the start of 2100,
//! accepted only by Azurite's basic validation.
//! Do not use this credential with real Azure services.

use async_trait::async_trait;
use azure_core::{
    base64,
    credentials::{AccessToken, TokenCredential, TokenRequestOptions},
    http::{RequestContent, Url},
    time::OffsetDateTime,
};
use azure_storage_blob::{
    models::{BlobClientGetPropertiesResultHeaders, BlobType},
    AppendBlobClient, BlobClient, BlobContainerClient, BlobServiceClient, BlockBlobClient,
    PageBlobClient,
};
use std::{error::Error, sync::Arc};

// Non-secret, unsigned token for Azurite only.
const SYNTHETIC_TOKEN: &str = concat!(
    // cspell:disable-next-line
    "eyJhbGciOiJub25lIiwidHlwIjoiSldUIn0.",
    "eyJpc3MiOiJodHRwczovL3N0cy53aW5kb3dzLm5ldC8wMDAwMDAwMC0wMDAwLTAwMDAtMDAwMC0wMDAwMDAwMDAwMDAvIiwiYXVkIjoiaHR0cHM6Ly9zdG9yYWdlLmF6dXJlLmNvbS8iLCJpYXQiOjE1Nzc4MzY4MDAsIm5iZiI6MTU3NzgzNjgwMCwiZXhwIjo0MTAyNDQ0ODAwfQ",
    ".",
);

#[derive(Debug)]
struct SyntheticCredential;

#[async_trait]
impl TokenCredential for SyntheticCredential {
    async fn get_token(
        &self,
        _scopes: &[&str],
        _options: Option<TokenRequestOptions<'_>>,
    ) -> azure_core::Result<AccessToken> {
        let expires_on = OffsetDateTime::from_unix_timestamp(4_102_444_800)
            .expect("2100-01-01 is a valid timestamp");
        Ok(AccessToken::new(SYNTHETIC_TOKEN, expires_on))
    }
}

#[tokio::test]
async fn synthetic_token_claims() -> Result<(), Box<dyn Error>> {
    let token = SyntheticCredential
        .get_token(&["https://storage.azure.com/.default"], None)
        .await?;
    assert_eq!(token.token.secret(), SYNTHETIC_TOKEN);
    let parts: Vec<_> = token.token.secret().split('.').collect();
    assert_eq!(parts.len(), 3);
    assert!(parts[2].is_empty(), "synthetic token must be unsigned");
    let header: serde_json::Value = serde_json::from_slice(&base64::decode_url_safe(parts[0])?)?;
    let claims: serde_json::Value = serde_json::from_slice(&base64::decode_url_safe(parts[1])?)?;
    assert_eq!(header["alg"], "none");
    assert_eq!(header["typ"], "JWT");
    assert_eq!(
        claims["iss"],
        "https://sts.windows.net/00000000-0000-0000-0000-000000000000/"
    );
    assert_eq!(claims["aud"], "https://storage.azure.com/");
    assert_eq!(claims["iat"], 1_577_836_800_i64);
    assert_eq!(claims["nbf"], 1_577_836_800_i64);
    assert_eq!(claims["exp"], 4_102_444_800_i64);
    assert_eq!(claims["exp"], token.expires_on.unix_timestamp());
    Ok(())
}

#[tokio::test]
#[ignore = "requires locally patched Azurite with --oauth basic"]
async fn upload_foo_get_properties() -> Result<(), Box<dyn Error>> {
    upload_foo_get_properties_at("http://127.0.0.1:10000/devstoreaccount1").await
}

#[tokio::test]
#[ignore = "requires Azurite with --oauth basic and a trusted HTTPS certificate"]
async fn upload_foo_get_properties_https() -> Result<(), Box<dyn Error>> {
    upload_foo_get_properties_at("https://127.0.0.1:10000/devstoreaccount1").await
}

async fn upload_foo_get_properties_at(endpoint: &str) -> Result<(), Box<dyn Error>> {
    let credential = Arc::new(SyntheticCredential);
    let service = BlobServiceClient::new(Url::parse(endpoint)?, Some(credential), None)?;
    let container_name = format!(
        "azurite-smoke-{}",
        OffsetDateTime::now_utc().unix_timestamp_nanos()
    );
    let container = service.blob_container_client(&container_name);
    container.create(None).await?;

    let blob = container.blob_client("foo");
    println!("Uploading bar to {}", blob.url());
    blob.upload(RequestContent::from(b"bar".to_vec()), None)
        .await?;

    let properties = blob.get_properties(None).await?;
    println!("Content length: {:?}", properties.content_length()?);
    println!("Blob type: {:?}", properties.blob_type()?);
    println!("ETag: {:?}", properties.etag()?);
    println!("Last modified: {:?}", properties.last_modified()?);
    assert_eq!(properties.content_length()?, Some(3));
    assert_eq!(properties.blob_type()?, Some(BlobType::BlockBlob));
    let content = blob.download(None).await?.body.collect().await?;
    assert_eq!(content.as_ref(), b"bar");
    Ok(())
}

#[test]
fn token_client_endpoint_validation() -> Result<(), Box<dyn Error>> {
    let credential = Arc::new(SyntheticCredential);
    for (endpoint, allowed) in [
        ("http://localhost:10000/devstoreaccount1", true),
        ("http://127.0.0.1:10000/devstoreaccount1", true),
        ("http://[::1]:10000/devstoreaccount1", true),
        ("https://example.com", true),
        ("http://example.com", false),
        ("http://localhost.example.com", false),
        ("http://192.168.1.1", false),
        ("ftp://localhost", false),
    ] {
        let url = Url::parse(endpoint)?;
        assert_eq!(
            BlobServiceClient::new(url.clone(), Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
        assert_eq!(
            BlobContainerClient::new(url.clone(), Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
        assert_eq!(
            BlobClient::new(url.clone(), Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
        assert_eq!(
            BlockBlobClient::new(url.clone(), Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
        assert_eq!(
            AppendBlobClient::new(url.clone(), Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
        assert_eq!(
            PageBlobClient::new(url, Some(credential.clone()), None).is_ok(),
            allowed,
            "{endpoint}"
        );
    }
    Ok(())
}
