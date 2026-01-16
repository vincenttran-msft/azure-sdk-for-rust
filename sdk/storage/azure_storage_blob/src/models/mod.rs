// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

pub(crate) mod content_range;
mod extensions;

use azure_core::fmt::SafeDebug;
use serde::{de::Error, Deserialize, Deserializer, Serialize, Serializer};

/// Internal representation of a blob name as it appears in XML.
#[derive(Clone, Default, Deserialize)]
struct BlobNameXml {
    /// The blob name content.
    #[serde(rename = "$text")]
    pub content: Option<String>,

    /// Whether the blob name is percent-encoded.
    #[serde(rename = "@Encoded")]
    pub encoded: Option<bool>,
}

/// A blob name that automatically handles percent-decoding during deserialization.
///
/// When deserializing from XML, if the `Encoded` attribute is `true`, the content
/// will be percent-decoded. Otherwise, the content is returned as-is.
///
/// This type is used via `@@alternateType` in TypeSpec to replace the generated
/// `BlobName` type, providing automatic decoding without needing `deserialize_with`.
#[derive(Clone, Default, SafeDebug)]
pub struct BlobName(pub Option<String>);

impl BlobName {
    /// Creates a new `BlobName` with the given content.
    pub fn new(content: impl Into<String>) -> Self {
        Self(Some(content.into()))
    }

    /// Returns the blob name as a string slice, if present.
    pub fn as_str(&self) -> Option<&str> {
        self.0.as_deref()
    }

    /// Consumes the `BlobName` and returns the inner `Option<String>`.
    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}

impl<'de> Deserialize<'de> for BlobName {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let xml = Option::<BlobNameXml>::deserialize(deserializer)?;

        let Some(xml) = xml else {
            return Ok(BlobName(None));
        };

        let Some(content) = xml.content else {
            return Ok(BlobName(None));
        };

        if xml.encoded.unwrap_or_default() {
            use percent_encoding::percent_decode_str;
            let decoded = percent_decode_str(&content)
                .decode_utf8()
                .map_err(D::Error::custom)?;
            Ok(BlobName(Some(decoded.into_owned())))
        } else {
            Ok(BlobName(Some(content)))
        }
    }
}

impl Serialize for BlobName {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.0.serialize(serializer)
    }
}
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
