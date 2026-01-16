// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub(crate) mod content_range;
mod extensions;

use azure_core::fmt::SafeDebug;
use serde::{de::Error, Deserialize, Deserializer, Serialize};

/// An Azure Storage Blob
#[derive(Clone, Default, Deserialize, SafeDebug, Serialize)]
#[non_exhaustive]
#[serde(rename = "Blob")]
pub struct BlobItemInternalCustom {
    /// The tags of the blob.
    #[serde(rename = "BlobTags", skip_serializing_if = "Option::is_none")]
    pub blob_tags: Option<BlobTags>,

    /// Whether the blob is deleted.
    #[serde(rename = "Deleted", skip_serializing_if = "Option::is_none")]
    pub deleted: Option<bool>,

    /// Whether the blob has versions only.
    #[serde(rename = "HasVersionsOnly", skip_serializing_if = "Option::is_none")]
    pub has_versions_only: Option<bool>,

    /// Whether the blob is the current version.
    #[serde(rename = "IsCurrentVersion", skip_serializing_if = "Option::is_none")]
    pub is_current_version: Option<bool>,

    /// The metadata of the blob.
    #[serde(rename = "Metadata", skip_serializing_if = "Option::is_none")]
    pub metadata: Option<BlobMetadata>,

    #[serde(
        rename = "Name",
        skip_serializing_if = "Option::is_none",
        deserialize_with = "crate::models::deserialize_blob_name",
        default
    )]
    pub name: Option<String>,

    /// The object replication metadata of the blob.
    #[serde(rename = "OrMetadata", skip_serializing_if = "Option::is_none")]
    pub object_replication_metadata: Option<ObjectReplicationMetadata>,

    /// The properties of the blob.
    #[serde(rename = "Properties", skip_serializing_if = "Option::is_none")]
    pub properties: Option<BlobPropertiesInternal>,

    /// The snapshot of the blob.
    #[serde(rename = "Snapshot", skip_serializing_if = "Option::is_none")]
    pub snapshot: Option<String>,

    /// The version id of the blob.
    #[serde(rename = "VersionId", skip_serializing_if = "Option::is_none")]
    pub version_id: Option<String>,
}

///
/// # Errors
///
/// Returns a deserialization error if the content is percent-encoded but contains
/// invalid UTF-8 sequences after decoding.
///
/// # Example
///
/// ```ignore
/// #[derive(Deserialize)]
/// struct MyStruct {
///     #[serde(deserialize_with = "deserialize_blob_name")]
///     name: Option<String>,
/// }
/// ```
pub fn deserialize_blob_name<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let blob_name = Option::<BlobName>::deserialize(deserializer)?;

    let Some(blob_name) = blob_name else {
        return Ok(None);
    };

    let Some(content) = blob_name.content else {
        return Ok(None);
    };

    if blob_name.encoded.unwrap_or_default() {
        use percent_encoding::percent_decode_str;
        let decoded = percent_decode_str(&content)
            .decode_utf8()
            .map_err(D::Error::custom)?;
        Ok(Some(decoded.into_owned()))
    } else {
        Ok(Some(content))
    }
}

use crate::generated::models::BlobName;
pub use crate::generated::models::{
    AccessPolicy, AccessTier, AccountKind, AppendBlobClientAppendBlockFromUrlOptions,
    AppendBlobClientAppendBlockFromUrlResult, AppendBlobClientAppendBlockFromUrlResultHeaders,
    AppendBlobClientAppendBlockOptions, AppendBlobClientAppendBlockResult,
    AppendBlobClientAppendBlockResultHeaders, AppendBlobClientCreateOptions,
    AppendBlobClientCreateResult, AppendBlobClientCreateResultHeaders, AppendBlobClientSealOptions,
    AppendBlobClientSealResult, AppendBlobClientSealResultHeaders, ArchiveStatus,
    BlobClientAcquireLeaseOptions, BlobClientAcquireLeaseResult,
    BlobClientAcquireLeaseResultHeaders, BlobClientBreakLeaseOptions, BlobClientBreakLeaseResult,
    BlobClientBreakLeaseResultHeaders, BlobClientChangeLeaseOptions, BlobClientChangeLeaseResult,
    BlobClientChangeLeaseResultHeaders, BlobClientCopyFromUrlResult,
    BlobClientCopyFromUrlResultHeaders, BlobClientCreateSnapshotOptions,
    BlobClientCreateSnapshotResult, BlobClientCreateSnapshotResultHeaders,
    BlobClientDeleteImmutabilityPolicyOptions, BlobClientDeleteOptions, BlobClientDownloadOptions,
    BlobClientDownloadResult, BlobClientDownloadResultHeaders, BlobClientGetAccountInfoOptions,
    BlobClientGetAccountInfoResult, BlobClientGetAccountInfoResultHeaders,
    BlobClientGetPropertiesOptions, BlobClientGetPropertiesResult,
    BlobClientGetPropertiesResultHeaders, BlobClientGetTagsOptions, BlobClientReleaseLeaseOptions,
    BlobClientReleaseLeaseResult, BlobClientReleaseLeaseResultHeaders, BlobClientRenewLeaseOptions,
    BlobClientRenewLeaseResult, BlobClientRenewLeaseResultHeaders, BlobClientSetExpiryResult,
    BlobClientSetExpiryResultHeaders, BlobClientSetImmutabilityPolicyOptions,
    BlobClientSetLegalHoldOptions, BlobClientSetMetadataOptions, BlobClientSetPropertiesOptions,
    BlobClientSetTagsOptions, BlobClientSetTierOptions, BlobClientStartCopyFromUrlResult,
    BlobClientStartCopyFromUrlResultHeaders, BlobClientUndeleteOptions,
    BlobContainerClientAcquireLeaseOptions, BlobContainerClientAcquireLeaseResult,
    BlobContainerClientAcquireLeaseResultHeaders, BlobContainerClientBreakLeaseOptions,
    BlobContainerClientBreakLeaseResult, BlobContainerClientBreakLeaseResultHeaders,
    BlobContainerClientChangeLeaseOptions, BlobContainerClientChangeLeaseResult,
    BlobContainerClientChangeLeaseResultHeaders, BlobContainerClientCreateOptions,
    BlobContainerClientDeleteOptions, BlobContainerClientFindBlobsByTagsOptions,
    BlobContainerClientGetAccessPolicyOptions, BlobContainerClientGetAccountInfoResult,
    BlobContainerClientGetAccountInfoResultHeaders, BlobContainerClientGetPropertiesOptions,
    BlobContainerClientGetPropertiesResult, BlobContainerClientGetPropertiesResultHeaders,
    BlobContainerClientListBlobFlatSegmentOptions, BlobContainerClientReleaseLeaseOptions,
    BlobContainerClientReleaseLeaseResult, BlobContainerClientReleaseLeaseResultHeaders,
    BlobContainerClientRenewLeaseOptions, BlobContainerClientRenewLeaseResult,
    BlobContainerClientRenewLeaseResultHeaders, BlobContainerClientSetAccessPolicyOptions,
    BlobContainerClientSetMetadataOptions, BlobCopySourceTags, BlobDeleteType, BlobExpiryOptions,
    BlobFlatListSegment, BlobItemInternal, BlobMetadata, BlobPropertiesInternal,
    BlobServiceClientFindBlobsByTagsOptions, BlobServiceClientGetAccountInfoOptions,
    BlobServiceClientGetAccountInfoResult, BlobServiceClientGetAccountInfoResultHeaders,
    BlobServiceClientGetPropertiesOptions, BlobServiceClientGetStatisticsOptions,
    BlobServiceClientListContainersSegmentOptions, BlobServiceClientSetPropertiesOptions,
    BlobServiceProperties, BlobTag, BlobTags, BlobType, Block,
    BlockBlobClientCommitBlockListOptions, BlockBlobClientCommitBlockListResult,
    BlockBlobClientCommitBlockListResultHeaders, BlockBlobClientGetBlockListOptions,
    BlockBlobClientQueryResult, BlockBlobClientQueryResultHeaders,
    BlockBlobClientStageBlockFromUrlResult, BlockBlobClientStageBlockFromUrlResultHeaders,
    BlockBlobClientStageBlockOptions, BlockBlobClientStageBlockResult,
    BlockBlobClientStageBlockResultHeaders, BlockBlobClientUploadBlobFromUrlOptions,
    BlockBlobClientUploadBlobFromUrlResult, BlockBlobClientUploadBlobFromUrlResultHeaders,
    BlockBlobClientUploadOptions, BlockBlobClientUploadResult, BlockBlobClientUploadResultHeaders,
    BlockList, BlockListHeaders, BlockListType, BlockLookupList, ContainerItem, CopyStatus,
    CorsRule, DeleteSnapshotsOptionType, EncryptionAlgorithmType, FileShareTokenIntent,
    FilterBlobItem, FilterBlobSegment, FilterBlobsIncludeItem, GeoReplication,
    GeoReplicationStatusType, ImmutabilityPolicyMode, LeaseDuration, LeaseState, LeaseStatus,
    ListBlobsFlatSegmentResponse, ListBlobsHierarchySegmentResponse, ListBlobsIncludeItem,
    ListContainersIncludeType, ListContainersSegmentResponse, Logging, Metrics,
    ObjectReplicationMetadata, PageBlobClientClearPagesOptions, PageBlobClientClearPagesResult,
    PageBlobClientClearPagesResultHeaders, PageBlobClientCopyIncrementalResult,
    PageBlobClientCopyIncrementalResultHeaders, PageBlobClientCreateOptions,
    PageBlobClientCreateResult, PageBlobClientCreateResultHeaders,
    PageBlobClientGetPageRangesOptions, PageBlobClientResizeOptions, PageBlobClientResizeResult,
    PageBlobClientResizeResultHeaders, PageBlobClientSetSequenceNumberOptions,
    PageBlobClientSetSequenceNumberResult, PageBlobClientSetSequenceNumberResultHeaders,
    PageBlobClientUploadPagesFromUrlOptions, PageBlobClientUploadPagesFromUrlResult,
    PageBlobClientUploadPagesFromUrlResultHeaders, PageBlobClientUploadPagesOptions,
    PageBlobClientUploadPagesResult, PageBlobClientUploadPagesResultHeaders, PageList,
    PageListHeaders, PremiumPageBlobAccessTier, PublicAccessType, QueryRequestType, QueryType,
    RehydratePriority, RetentionPolicy, SequenceNumberActionType, SignedIdentifier,
    SignedIdentifiers, SignedIdentifiersHeaders, SkuName, StaticWebsite, StorageErrorCode,
    StorageServiceStats, UserDelegationKey,
};
