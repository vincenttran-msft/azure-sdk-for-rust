// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use azure_core::{
    http::{RequestContent, StatusCode},
    time::OffsetDateTime,
};
use azure_core_test::{recorded, TestContext};
use azure_storage_blob::{
    format_page_range,
    models::{
        AccessPolicy, AppendBlobClientAppendBlockOptions, AppendBlobClientCreateOptions,
        AppendBlobClientCreateResultHeaders, BlobClientCreateSnapshotOptions,
        BlobClientDeleteOptions, BlobClientGetPropertiesOptions,
        BlobClientGetPropertiesResultHeaders, BlobClientSetMetadataOptions,
        BlobClientSetPropertiesOptions, BlobClientSetTagsOptions, BlobContainerClientDeleteOptions,
        BlobContainerClientSetAccessPolicyOptions, BlobContainerClientSetMetadataOptions, BlobTags,
        BlockBlobClientCommitBlockListOptions, BlockBlobClientCommitBlockListResultHeaders,
        BlockBlobClientGetBlockListOptions, BlockBlobClientUploadOptions,
        BlockBlobClientUploadResultHeaders, BlockLookupList, DeleteSnapshotsOptionType,
        PageBlobClientClearPagesOptions, PageBlobClientCreateOptions,
        PageBlobClientCreateResultHeaders, PageBlobClientGetPageRangesOptions,
        PageBlobClientSetSequenceNumberOptions, PageBlobClientUploadPagesOptions,
        SequenceNumberActionType, SignedIdentifiers,
    },
};
use azure_storage_blob_test::{
    create_test_blob, get_blob_name, get_container_client, StorageAccount,
};
use std::{collections::HashMap, error::Error};
use time::Duration;

#[recorded::test]
async fn test_append_blob_create_if_none_match_star(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-ifnm-star", get_blob_name(recording)));
    let append_blob_client = blob_client.append_blob_client();

    // First Create Succeeds
    let options = AppendBlobClientCreateOptions {
        if_none_match: Some("*".to_string()),
        ..Default::default()
    };
    append_blob_client.create(Some(options)).await?;

    // Second Create Fails (Blob Already Exists)
    let options = AppendBlobClientCreateOptions {
        if_none_match: Some("*".to_string()),
        ..Default::default()
    };
    let result = append_blob_client.create(Some(options)).await;
    let status = result.unwrap_err().http_status();
    assert_eq!(Some(StatusCode::Conflict), status);

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_append_block_etag_and_date_conditions(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&format!("{}-etag", get_blob_name(recording)));
    let append_blob_client = blob_client.append_blob_client();

    let create_response = append_blob_client.create(None).await?;
    let etag = create_response.etag()?.unwrap().to_string();

    // If-Match Success
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"data".to_vec()),
            4,
            Some(AppendBlobClientAppendBlockOptions {
                if_match: Some(etag.clone()),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // If-Match Failure - bogus ETag
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"fail".to_vec()),
            4,
            Some(AppendBlobClientAppendBlockOptions {
                if_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Unmodified-Since Success - far future
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"ok".to_vec()),
            2,
            Some(AppendBlobClientAppendBlockOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // If-Unmodified-Since Failure - far past
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"fail".to_vec()),
            4,
            Some(AppendBlobClientAppendBlockOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Success - bogus ETag means "no match", so operation proceeds
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"ok".to_vec()),
            2,
            Some(AppendBlobClientAppendBlockOptions {
                if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // Capture Fresh ETag After Successful Append
    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    // If-None-Match Failure - current ETag matches, so operation is rejected
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"fail".to_vec()),
            4,
            Some(AppendBlobClientAppendBlockOptions {
                if_none_match: Some(etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Modified-Since Success - blob was definitely modified after far_past
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"ok2".to_vec()),
            3,
            Some(AppendBlobClientAppendBlockOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // If-Modified-Since Failure - blob has not been modified since far_future
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"fail2".to_vec()),
            5,
            Some(AppendBlobClientAppendBlockOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_append_block_if_tags(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&format!("{}-iftags", get_blob_name(recording)));
    let append_blob_client = blob_client.append_blob_client();
    append_blob_client.create(None).await?;

    let tags = HashMap::from([("env".to_string(), "test".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    // Matching if_tags succeeds
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"tagged".to_vec()),
            6,
            Some(AppendBlobClientAppendBlockOptions {
                if_tags: Some("\"env\" = 'test'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // Non-matching if_tags fails with 412
    let result = append_blob_client
        .append_block(
            RequestContent::from(b"fail".to_vec()),
            4,
            Some(AppendBlobClientAppendBlockOptions {
                if_tags: Some("\"env\" = 'prod'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_get_blob_properties_etag_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    // If-Match Success
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_match: Some(etag.clone()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-Match Failure
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Success (bogus ETag means no match, so request proceeds)
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-None-Match Failure (current ETag matches, so a GET-like request returns 304)
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_none_match: Some(etag),
            ..Default::default()
        }))
        .await;
    let status = result.unwrap_err().http_status();
    assert_eq!(Some(StatusCode::NotModified), status);

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_get_blob_properties_date_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    // If-Modified-Since Success - blob was modified after far_past
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-Modified-Since Failure - blob has not been modified since far_future
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    let status = result.unwrap_err().http_status();
    assert_eq!(Some(StatusCode::NotModified), status);

    // If-Unmodified-Since Success - blob has not been modified since far_future
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-Unmodified-Since Failure - blob was modified after far_past
    let result = blob_client
        .get_properties(Some(BlobClientGetPropertiesOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_set_blob_metadata_access_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    let metadata = HashMap::from([("key1".to_string(), "value1".to_string())]);

    // If-Unmodified-Since Success
    blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // If-Unmodified-Since Failure
    let result = blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Match Failure - ETag is stale after the successful set_metadata above
    let result = blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_match: Some(etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Modified-Since Success
    blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // If-Modified-Since Failure
    let result = blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Success with bogus ETag
    blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    // If-None-Match Failure with current ETag
    let current_props = blob_client.get_properties(None).await?;
    let current_etag = current_props.etag()?.unwrap().to_string();
    let result = blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_none_match: Some(current_etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Tags Success/Failure
    let tags = HashMap::from([("env".to_string(), "dev".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_tags: Some("\"env\" = 'dev'".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    let result = blob_client
        .set_metadata(
            &metadata,
            Some(BlobClientSetMetadataOptions {
                if_tags: Some("\"env\" = 'prod'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_set_blob_properties_access_conditions(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    // If-Match Success
    blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("text/plain".to_string()),
            if_match: Some(etag.clone()),
            ..Default::default()
        }))
        .await?;

    // If-Match Failure - ETag has changed after the successful set_properties above
    let result = blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("text/html".to_string()),
            if_match: Some(etag),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Modified-Since Success
    blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_language: Some("en".to_string()),
            if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await?;

    // If-Modified-Since Failure
    let result = blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_language: Some("fr".to_string()),
            if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Unmodified-Since Success
    blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_language: Some("de".to_string()),
            if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await?;

    // If-Unmodified-Since Failure
    let result = blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_language: Some("it".to_string()),
            if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Success with bogus ETag
    blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("application/json".to_string()),
            if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await?;

    // If-None-Match Failure with current ETag
    let current_props = blob_client.get_properties(None).await?;
    let current_etag = current_props.etag()?.unwrap().to_string();
    let result = blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("application/xml".to_string()),
            if_none_match: Some(current_etag),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Tags Success/Failure
    let tags = HashMap::from([("stage".to_string(), "blue".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("text/markdown".to_string()),
            if_tags: Some("\"stage\" = 'blue'".to_string()),
            ..Default::default()
        }))
        .await?;

    let result = blob_client
        .set_properties(Some(BlobClientSetPropertiesOptions {
            blob_content_type: Some("text/csv".to_string()),
            if_tags: Some("\"stage\" = 'green'".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_blob_if_tags_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&format!("{}-iftags", get_blob_name(recording)));
    create_test_blob(&blob_client, None, None).await?;

    let tags = HashMap::from([("stage".to_string(), "production".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    // Set-Tags If-Tags Match Success
    let new_tags = HashMap::from([("stage".to_string(), "staging".to_string())]);
    blob_client
        .set_tags(
            RequestContent::try_from(BlobTags::from(new_tags))?,
            Some(BlobClientSetTagsOptions {
                if_tags: Some("\"stage\" = 'production'".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    // Set-Tags If-Tags Mismatch Failure (tag was just changed to "staging")
    let final_tags = HashMap::from([("stage".to_string(), "archive".to_string())]);
    let result = blob_client
        .set_tags(
            RequestContent::try_from(BlobTags::from(final_tags))?,
            Some(BlobClientSetTagsOptions {
                if_tags: Some("\"stage\" = 'production'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Delete If-Tags Mismatch Failure
    let result = blob_client
        .delete(Some(BlobClientDeleteOptions {
            if_tags: Some("\"stage\" = 'production'".to_string()),
            delete_snapshots: Some(DeleteSnapshotsOptionType::Include),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Delete If-Tags Match Success
    blob_client
        .delete(Some(BlobClientDeleteOptions {
            if_tags: Some("\"stage\" = 'staging'".to_string()),
            delete_snapshots: Some(DeleteSnapshotsOptionType::Include),
            ..Default::default()
        }))
        .await?;

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_create_snapshot_access_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client = container_client.blob_client(&get_blob_name(recording));
    create_test_blob(&blob_client, None, None).await?;

    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    // If-Match Success
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_match: Some(etag.clone()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-Match Failure
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Failure (current ETag matches, request rejected)
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_none_match: Some(etag),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-None-Match Success with bogus ETag
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // If-Modified-Since Success/failure
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Unmodified-Since Success/failure
    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Tags Success/Failure
    let tags = HashMap::from([("kind".to_string(), "stateful".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_tags: Some("\"kind\" = 'stateful'".to_string()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    let result = blob_client
        .create_snapshot(Some(BlobClientCreateSnapshotOptions {
            if_tags: Some("\"kind\" = 'stateless'".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_container_set_metadata_if_modified_since(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    // If-Modified-Since Success - container was created after far_past
    container_client
        .set_metadata(
            &HashMap::from([("key".to_string(), "val".to_string())]),
            Some(BlobContainerClientSetMetadataOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // If-Modified-Since Failure - container has not been modified since far_future
    let result = container_client
        .set_metadata(
            &HashMap::from([("key2".to_string(), "val2".to_string())]),
            Some(BlobContainerClientSetMetadataOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_container_set_access_policy_date_conditions(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    // Set-Access-Policy If-Unmodified-Since Success
    let empty_acl: SignedIdentifiers = HashMap::<String, AccessPolicy>::new().into();
    container_client
        .set_access_policy(
            RequestContent::try_from(empty_acl)?,
            Some(BlobContainerClientSetAccessPolicyOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // Set-Access-Policy If-Unmodified-Since Failure
    let empty_acl2: SignedIdentifiers = HashMap::<String, AccessPolicy>::new().into();
    let result = container_client
        .set_access_policy(
            RequestContent::try_from(empty_acl2)?,
            Some(BlobContainerClientSetAccessPolicyOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Set-Access-Policy If-Modified-Since Success
    let empty_acl3: SignedIdentifiers = HashMap::<String, AccessPolicy>::new().into();
    container_client
        .set_access_policy(
            RequestContent::try_from(empty_acl3)?,
            Some(BlobContainerClientSetAccessPolicyOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // Set-Access-Policy If-Modified-Since Failure
    let empty_acl4: SignedIdentifiers = HashMap::<String, AccessPolicy>::new().into();
    let result = container_client
        .set_access_policy(
            RequestContent::try_from(empty_acl4)?,
            Some(BlobContainerClientSetAccessPolicyOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;

    Ok(())
}

#[recorded::test]
async fn test_container_delete_date_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;

    // Delete If-Modified-Since Failure - container has not been modified since far_future
    let result = container_client
        .delete(Some(BlobContainerClientDeleteOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Delete If-Modified-Since Success - container was created after far_past
    container_client
        .delete(Some(BlobContainerClientDeleteOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await?;

    Ok(())
}

#[recorded::test]
async fn test_block_blob_upload_if_none_match_star(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-ifnm-star", get_blob_name(recording)));

    // First upload with if_none_match: * succeeds (blob does not yet exist)
    blob_client
        .upload(
            RequestContent::from(b"first".to_vec()),
            false,
            5,
            Some(BlockBlobClientUploadOptions {
                if_none_match: Some("*".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    // Second upload with if_none_match: * fails (blob now exists)
    let result = blob_client
        .upload(
            RequestContent::from(b"second".to_vec()),
            false,
            6,
            Some(BlockBlobClientUploadOptions {
                if_none_match: Some("*".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::Conflict),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_commit_block_list_access_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-cbl-match", get_blob_name(recording)));
    let block_blob_client = blob_client.block_blob_client();

    // Stage a block so we can commit it
    let block_id = b"block-1".to_vec();
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;

    // First commit - no conditions; captures ETag from response
    let commit_response = block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            None,
        )
        .await?;
    let etag_v1 = commit_response.etag()?.unwrap().to_string();

    // Stage block again for second commit
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;

    // Second commit with stale ETag should fail
    let result = block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Stage block again since uncommitted list was consumed by the failed attempt
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;

    // Second commit with correct ETag succeeds
    block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_match: Some(etag_v1),
                ..Default::default()
            }),
        )
        .await?;

    // Commit If-None-Match Success
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    // Commit If-None-Match Failure against current ETag
    let props = blob_client.get_properties(None).await?;
    let current_etag = props.etag()?.unwrap().to_string();
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    let result = block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_none_match: Some(current_etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Commit If-Modified-Since Success/Failure
    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    let result = block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Commit If-Tags Success/Failure
    let tags = HashMap::from([("mode".to_string(), "cbl".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_tags: Some("\"mode\" = 'cbl'".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    block_blob_client
        .stage_block(
            &block_id,
            7,
            RequestContent::from(b"content".to_vec()),
            None,
        )
        .await?;
    let result = block_blob_client
        .commit_block_list(
            BlockLookupList {
                committed: Some(Vec::new()),
                latest: Some(vec![block_id.clone()]),
                uncommitted: Some(Vec::new()),
            }
            .try_into()?,
            Some(BlockBlobClientCommitBlockListOptions {
                if_tags: Some("\"mode\" = 'other'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_block_blob_upload_access_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-bb-cond", get_blob_name(recording)));

    // Upload initial blob to get an ETag
    let upload_response = blob_client
        .upload(RequestContent::from(b"initial".to_vec()), false, 7, None)
        .await?;
    let etag = upload_response.etag()?.unwrap().to_string();

    // If-Match Success
    blob_client
        .upload(
            RequestContent::from(b"updated".to_vec()),
            true,
            7,
            Some(BlockBlobClientUploadOptions {
                if_match: Some(etag.clone()),
                ..Default::default()
            }),
        )
        .await?;

    // If-Match Failure (stale)
    let result = blob_client
        .upload(
            RequestContent::from(b"fail".to_vec()),
            true,
            4,
            Some(BlockBlobClientUploadOptions {
                if_match: Some(etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Unmodified-Since Failure
    let result = blob_client
        .upload(
            RequestContent::from(b"fail2".to_vec()),
            true,
            5,
            Some(BlockBlobClientUploadOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_block_blob_get_block_list_if_tags(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-bb-tags", get_blob_name(recording)));
    let block_blob_client = blob_client.block_blob_client();

    // Upload initial blob
    blob_client
        .upload(RequestContent::from(b"initial".to_vec()), false, 7, None)
        .await?;

    // get_block_list with if_tags: set tags first
    let tags = HashMap::from([("tier".to_string(), "hot".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    // Stage a block so get_block_list has something to return
    block_blob_client
        .stage_block(b"blk1", 4, RequestContent::from(b"blk1".to_vec()), None)
        .await?;

    // Matching if_tags on get_block_list succeeds
    let result = block_blob_client
        .get_block_list(
            azure_storage_blob::models::BlockListType::All,
            Some(BlockBlobClientGetBlockListOptions {
                if_tags: Some("\"tier\" = 'hot'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert!(result.is_ok());

    // Non-matching if_tags on get_block_list fails
    let result = block_blob_client
        .get_block_list(
            azure_storage_blob::models::BlockListType::All,
            Some(BlockBlobClientGetBlockListOptions {
                if_tags: Some("\"tier\" = 'cold'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_page_blob_create_if_none_match_star(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-pb-ifnm", get_blob_name(recording)));
    let page_blob_client = blob_client.page_blob_client();

    // First create succeeds
    page_blob_client
        .create(
            512,
            Some(PageBlobClientCreateOptions {
                if_none_match: Some("*".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    // Second create on same blob fails
    let result = page_blob_client
        .create(
            512,
            Some(PageBlobClientCreateOptions {
                if_none_match: Some("*".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::Conflict),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_upload_pages_etag_and_date_conditions(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-pb-etag", get_blob_name(recording)));
    let page_blob_client = blob_client.page_blob_client();

    let create_response = page_blob_client.create(512, None).await?;
    let etag = create_response.etag()?.unwrap().to_string();

    // If-Match Success
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'A'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_match: Some(etag.clone()),
                ..Default::default()
            }),
        )
        .await?;

    // If-Match Failure (ETag has changed after the upload above)
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'B'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_match: Some(etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Unmodified-Since Success
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'C'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // If-Unmodified-Since Failure
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'D'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Modified-Since Success
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'E'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    // If-Modified-Since Failure
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'F'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_page_blob_sequence_number_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-pb-seqno", get_blob_name(recording)));
    let page_blob_client = blob_client.page_blob_client();

    // Create with initial sequence number 5
    page_blob_client
        .create(
            512,
            Some(PageBlobClientCreateOptions {
                blob_sequence_number: Some(5),
                ..Default::default()
            }),
        )
        .await?;

    // If-Sequence-Number-Equal-To Success (value == 5)
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'A'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_equal_to: Some(5),
                ..Default::default()
            }),
        )
        .await?;

    // If-Sequence-Number-Equal-To Failure (value != 5)
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'B'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_equal_to: Some(99),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Sequence-Number-Less-Than Success (5 < 10)
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'C'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_less_than: Some(10),
                ..Default::default()
            }),
        )
        .await?;

    // If-Sequence-Number-Less-Than Failure (5 is not < 1)
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'D'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_less_than: Some(1),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // If-Sequence-Number-Less-Than-Or-Equal-To Success (5 <= 5)
    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'E'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_less_than_or_equal_to: Some(5),
                ..Default::default()
            }),
        )
        .await?;

    // If-Sequence-Number-Less-Than-Or-Equal-To Failure (5 is not <= 4)
    let result = page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'F'; 512]),
            512,
            format_page_range(0, 512)?,
            Some(PageBlobClientUploadPagesOptions {
                if_sequence_number_less_than_or_equal_to: Some(4),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Sequence number conditions on clear_pages - equal_to success
    page_blob_client
        .clear_pages(
            format_page_range(0, 512)?,
            Some(PageBlobClientClearPagesOptions {
                if_sequence_number_equal_to: Some(5),
                ..Default::default()
            }),
        )
        .await?;

    // Sequence number conditions on clear_pages - equal_to failure
    let result = page_blob_client
        .clear_pages(
            format_page_range(0, 512)?,
            Some(PageBlobClientClearPagesOptions {
                if_sequence_number_equal_to: Some(99),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_page_blob_get_page_ranges_conditions(ctx: TestContext) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-pb-ranges", get_blob_name(recording)));
    let page_blob_client = blob_client.page_blob_client();

    page_blob_client.create(512, None).await?;

    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'A'; 512]),
            512,
            format_page_range(0, 512)?,
            None,
        )
        .await?;

    // Capture Fresh ETag After upload_pages
    let props = blob_client.get_properties(None).await?;
    let etag = props.etag()?.unwrap().to_string();

    // Get-Page-Ranges If-Match Success
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_match: Some(etag.clone()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // Get-Page-Ranges If-Match Failure (blob changed after upload_pages)
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Get-Page-Ranges If-None-Match Success with bogus ETag
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // Get-Page-Ranges If-None-Match Failure with current ETag (GET returns 304)
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_none_match: Some(etag.clone()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::NotModified),
        result.unwrap_err().http_status()
    );

    // Get-Page-Ranges If-Modified-Since Success/Failure
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_modified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::NotModified),
        result.unwrap_err().http_status()
    );

    // Get-Page-Ranges If-Unmodified-Since Success/Failure
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Set tags for if_tags tests
    let tags = HashMap::from([("type".to_string(), "page".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    // Get-Page-Ranges If-Tags Match Success
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_tags: Some("\"type\" = 'page'".to_string()),
            ..Default::default()
        }))
        .await;
    assert!(result.is_ok());

    // Get-Page-Ranges If-Tags Mismatch Failure
    let result = page_blob_client
        .get_page_ranges(Some(PageBlobClientGetPageRangesOptions {
            if_tags: Some("\"type\" = 'block'".to_string()),
            ..Default::default()
        }))
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}

#[recorded::test]
async fn test_page_blob_set_sequence_number_conditions(
    ctx: TestContext,
) -> Result<(), Box<dyn Error>> {
    // Recording Setup
    let recording = ctx.recording();
    let container_client =
        get_container_client(recording, true, StorageAccount::Standard, None).await?;
    let blob_client =
        container_client.blob_client(&format!("{}-pb-seq-conditions", get_blob_name(recording)));
    let page_blob_client = blob_client.page_blob_client();

    page_blob_client.create(512, None).await?;

    page_blob_client
        .upload_pages(
            RequestContent::from(vec![b'A'; 512]),
            512,
            format_page_range(0, 512)?,
            None,
        )
        .await?;

    let tags = HashMap::from([("type".to_string(), "page".to_string())]);
    blob_client
        .set_tags(RequestContent::try_from(BlobTags::from(tags))?, None)
        .await?;

    // Set-Sequence-Number If-Match Success
    let props = blob_client.get_properties(None).await?;
    let current_etag = props.etag()?.unwrap().to_string();
    page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_match: Some(current_etag),
                ..Default::default()
            }),
        )
        .await?;

    // Set-Sequence-Number If-Match Failure
    let result = page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Set-Sequence-Number If-None-Match Success/Failure
    page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_none_match: Some("\"00000000000000000000000000000000\"".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    let props = blob_client.get_properties(None).await?;
    let current_etag = props.etag()?.unwrap().to_string();
    let result = page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_none_match: Some(current_etag),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Set-Sequence-Number If-Unmodified-Since Success/Failure
    page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() + Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await?;

    let result = page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_unmodified_since: Some(OffsetDateTime::now_utc() - Duration::days(3650)),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    // Set-Sequence-Number If-Tags Success/Failure
    page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_tags: Some("\"type\" = 'page'".to_string()),
                ..Default::default()
            }),
        )
        .await?;

    let result = page_blob_client
        .set_sequence_number(
            SequenceNumberActionType::Increment,
            Some(PageBlobClientSetSequenceNumberOptions {
                if_tags: Some("\"type\" = 'block'".to_string()),
                ..Default::default()
            }),
        )
        .await;
    assert_eq!(
        Some(StatusCode::PreconditionFailed),
        result.unwrap_err().http_status()
    );

    container_client.delete(None).await?;
    Ok(())
}
