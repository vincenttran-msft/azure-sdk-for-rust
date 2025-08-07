// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::http::RequestContent;
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
    let hns_file_client = container_client.file_hns_client(blob_client_1.blob_name().into());
    hns_file_client.create(None).await?;
    // Append & Flush
    let data = b"abc";
    hns_file_client
        .append_data(RequestContent::from(data.to_vec()), 0, 3, None)
        .await?;
    hns_file_client.flush_data(3, None).await?;

    // HNS Directory Client
    let hns_directory_client =
        container_client.directory_hns_client(blob_client_2.blob_name().into());
    hns_directory_client.create(None).await?;
    // Rename
    hns_directory_client
        .rename_directory("novel".to_string(), None)
        .await?;

    Ok(())
}
