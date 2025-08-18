// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::http::RequestContent;
use azure_core_test::{recorded, TestContext};
use azure_storage_blob::models::HierarchicalClientSetAccessControlOptions;
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
    let hns_file_client = container_client.file_client(blob_client_1.blob_name().into());
    hns_file_client.create(None).await?;
    // Append & Flush
    let data = b"abc";
    hns_file_client
        .append_data(RequestContent::from(data.to_vec()), 0, 3, None)
        .await?;
    hns_file_client.flush_data(3, None).await?;

    // HNS Directory Client
    let hns_directory_client = container_client.directory_client(blob_client_2.blob_name().into());
    hns_directory_client.create(None).await?;
    // Rename
    hns_directory_client
        .rename_directory("novel".to_string(), None)
        .await?;

    Ok(())
}

#[recorded::test]
async fn sample(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Resources
    let art = r#"_._     _,-'""`-._
    (,-.`._,'(       |\`-/|
    `-.-' \ )-`( , o o)
          `-    \`_`"'-"#;
    let data = b"1.A 2.B 3.C 4.D";
    let recording = ctx.recording();

    // Container Setup
    let container_client = get_container_client(recording, false).await?;
    container_client.create_container(None).await?;

    // Setup Directory Structure (Pictures, Documents)
    let pictures_dir_client = container_client.directory_client("Pictures".into());
    pictures_dir_client.create(None).await?;

    let documents_dir_client = container_client.directory_client("Documents".into());
    documents_dir_client.create(None).await?;

    // Add "cat.txt" to Pictures
    let dir_client_2025 = pictures_dir_client.sub_directory("2025".into());
    dir_client_2025.create(None).await?;
    let cat_client = dir_client_2025.file_client("cat.txt".into()).blob_client();

    cat_client
        .upload(
            RequestContent::try_from(art)?,
            false,
            art.len() as u64,
            None,
        )
        .await?;

    // Add "homework.txt" to Documents
    let homework_blob_client = documents_dir_client
        .file_client("homework.txt".into())
        .blob_client();
    homework_blob_client
        .upload(
            RequestContent::from(data.to_vec()),
            false,
            data.len() as u64,
            None,
        )
        .await?;

    // Change ACLs to "homework.txt"
    let set_acl_options = HierarchicalClientSetAccessControlOptions {
        permissions: Some("0777".to_string()),
        ..Default::default()
    };
    let homework_file_client = container_client
        .directory_client("Documents".into())
        .file_client("homework.txt".into());
    homework_file_client
        .set_access_control(Some(set_acl_options))
        .await?;

    Ok(())
}
