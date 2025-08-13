// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::{http::RequestContent, Bytes};
use azure_core_test::{recorded, TestContext};
use azure_storage_blob_test::{get_blob_name, get_container_client};
use std::error::Error;

#[recorded::test]
async fn test(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client = get_container_client(recording, false).await?;
    container_client.create_container(None).await?;
    let blob_client_1 = container_client.blob_client(get_blob_name(recording));
    let blob_client_2 = container_client.blob_client("singleton".to_string());

    // HNS File Client
    let hns_file_client = blob_client_1.file_hns_client();
    hns_file_client.create(None).await?;
    // Append & Flush
    let data = b"abc";
    hns_file_client
        .append_data(RequestContent::from(data.to_vec()), 0, 3, None)
        .await?;
    hns_file_client.flush_data(3, None).await?;

    // HNS Directory Client
    let hns_directory_client = blob_client_2.directory_hns_client();
    hns_directory_client.create(None).await?;
    // Rename
    hns_directory_client
        .rename_directory("novel".to_string(), None)
        .await?;

    Ok(())
}

#[recorded::test]
async fn contrived_example(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup

    use azure_storage_blob::models::HierarchicalClientSetAccessControlOptions;
    let recording = ctx.recording();

    // Executes basic hierarchical storage operations:
    // - [1] Create a directory
    // - [2] Upload a file
    // - [3] Set file ACL
    // - [4] Download the file

    // Observations while implementing:
    // Will probably need a way to go from Blob <-> Hierarchical Client to avoid re-traversal down file structure
    // i.e. in [2] if I want to avoid AP&F and use Blob client, I have to manually get a Blob client pointing at the same path (inside of newly created dir)

    // - [1] Create a directory
    let container_client = get_container_client(recording, false).await?;
    container_client.create_container(None).await?;
    let blob_client = container_client.blob_client(get_blob_name(recording));
    let directory_client = blob_client.directory_hns_client();
    directory_client.create(None).await?;

    let data: &'static [u8; 3] = b"abc";
    // - [2] Upload a file
    let file_client = directory_client.file_client("test_file".to_string());
    file_client.create(None).await?;
    file_client
        .append_data(RequestContent::from(data.to_vec()), 0, 3, None)
        .await?;
    file_client.flush_data(3, None).await?;

    // - [3] Set file ACL
    let set_acl_options = HierarchicalClientSetAccessControlOptions {
        permissions: Some("0777".to_string()),
        ..Default::default()
    };
    file_client
        .set_access_control(Some(set_acl_options))
        .await?;

    // - [4] Download the file
    let download_response = file_client.download(None).await?;
    let response = download_response.into_body().collect().await?;
    assert_eq!(Bytes::from_static(data), response);

    Ok(())
}
