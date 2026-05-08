// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

use crate::models::{
    method_options::BlockBlobClientUploadOptions, AccessPolicy, AppendBlobClientCreateOptions,
    BlobTag, BlobTags, BlockBlobClientCommitBlockListOptions,
    BlockBlobClientUploadBlobFromUrlOptions, PageBlobClientCreateOptions, SignedIdentifier,
    SignedIdentifiers,
};
use percent_encoding::{percent_encode, NON_ALPHANUMERIC};
use std::collections::HashMap;

/// Converts a [`BlobTags`] to the `x-ms-tags` header string format (`key=value&key2=value2`).
///
/// Keys and values are assumed to be **raw** (not pre-encoded) and are percent-encoded here so
/// that `&` and `=` in user data can't be confused with the header's separators. Returns `None`
/// if there are no complete (key, value) tag entries.
///
/// The `BlobTags` type carries raw values everywhere — values returned by `get_blob_tags` are
/// raw, values constructed from a `HashMap` via [`From`] are raw, and values built via struct
/// literal are raw. Encoding happens **only** at this header boundary; the XML body path used
/// by `set_blob_tags` does not call this function.
pub(crate) fn blob_tags_to_string(tags: &BlobTags) -> Option<String> {
    let result = match &tags.blob_tag_set {
        Some(tag_set) => tag_set
            .iter()
            .filter_map(|tag| match (&tag.key, &tag.value) {
                (Some(k), Some(v)) => {
                    let encoded_key = percent_encode(k.as_bytes(), NON_ALPHANUMERIC);
                    let encoded_value = percent_encode(v.as_bytes(), NON_ALPHANUMERIC);
                    Some(format!("{}={}", encoded_key, encoded_value))
                }
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("&"),
        None => String::new(),
    };
    if result.is_empty() {
        None
    } else {
        Some(result)
    }
}

/// Augments the current options bag to only create if the Page blob does not already exist.
/// # Arguments
///
/// * `self` - The options bag to be modified.
impl PageBlobClientCreateOptions<'_> {
    pub fn if_not_exists(self) -> Self {
        Self {
            if_none_match: Some("*".into()),
            ..self
        }
    }
}

/// Augments the current options bag to only create if the Append blob does not already exist.
/// # Arguments
///
/// * `self` - The options bag to be modified.
impl AppendBlobClientCreateOptions<'_> {
    pub fn if_not_exists(self) -> Self {
        Self {
            if_none_match: Some("*".into()),
            ..self
        }
    }
}

/// Augments the current options bag to only create if the Block blob does not already exist.
/// # Arguments
///
/// * `self` - The options bag to be modified.
impl BlockBlobClientUploadBlobFromUrlOptions<'_> {
    pub fn if_not_exists(self) -> Self {
        Self {
            if_none_match: Some("*".into()),
            ..self
        }
    }
}

/// Augments the current options bag to only create if the Block blob does not already exist.
/// # Arguments
///
/// * `self` - The options bag to be modified.
impl BlockBlobClientUploadOptions<'_> {
    pub fn if_not_exists(self) -> Self {
        Self {
            if_none_match: Some("*".into()),
            ..self
        }
    }
}

/// Converts a [`BlobTags`] into a `HashMap<String, String>`.
///
/// Keys and values are passed through verbatim — `BlobTags` carries raw, unencoded strings
/// (see [`blob_tags_to_string`] for the invariant). Tag entries with a missing key or value
/// are skipped.
impl From<BlobTags> for HashMap<String, String> {
    fn from(blob_tags: BlobTags) -> Self {
        let mut map = HashMap::new();

        if let Some(tags) = blob_tags.blob_tag_set {
            for tag in tags {
                if let (Some(key), Some(value)) = (tag.key, tag.value) {
                    map.insert(key, value);
                }
            }
        }
        map
    }
}

/// Converts a `HashMap<String, String>` into a [`BlobTags`].
///
/// Keys and values are stored **raw** — do not pre-encode them. The SDK percent-encodes only
/// when serializing to the `x-ms-tags` header (see [`blob_tags_to_string`]).
impl From<HashMap<String, String>> for BlobTags {
    fn from(tags: HashMap<String, String>) -> Self {
        let blob_tags = tags
            .into_iter()
            .map(|(k, v)| BlobTag {
                key: Some(k),
                value: Some(v),
            })
            .collect();
        BlobTags {
            blob_tag_set: Some(blob_tags),
        }
    }
}

/// Sets blob tags on the options bag, encoding them for the `x-ms-tags` header.
///
/// Pass raw (unencoded) tag keys and values; the SDK percent-encodes them for transport.
impl AppendBlobClientCreateOptions<'_> {
    pub fn with_tags(mut self, tags: &BlobTags) -> Self {
        self.blob_tags_string = blob_tags_to_string(tags);
        self
    }
}

/// Sets blob tags on the options bag, encoding them for the `x-ms-tags` header.
///
/// Pass raw (unencoded) tag keys and values; the SDK percent-encodes them for transport.
impl PageBlobClientCreateOptions<'_> {
    pub fn with_tags(mut self, tags: &BlobTags) -> Self {
        self.blob_tags_string = blob_tags_to_string(tags);
        self
    }
}

/// Sets blob tags on the options bag, encoding them for the `x-ms-tags` header.
///
/// Pass raw (unencoded) tag keys and values; the SDK percent-encodes them for transport.
impl BlockBlobClientUploadBlobFromUrlOptions<'_> {
    pub fn with_tags(mut self, tags: &BlobTags) -> Self {
        self.blob_tags_string = blob_tags_to_string(tags);
        self
    }
}

/// Sets blob tags on the options bag, encoding them for the `x-ms-tags` header.
///
/// Pass raw (unencoded) tag keys and values; the SDK percent-encodes them for transport.
impl BlockBlobClientCommitBlockListOptions<'_> {
    pub fn with_tags(mut self, tags: &BlobTags) -> Self {
        self.blob_tags_string = blob_tags_to_string(tags);
        self
    }
}

/// Converts a `HashMap<String, AccessPolicy>` into a `SignedIdentifiers` struct.
impl From<HashMap<String, AccessPolicy>> for SignedIdentifiers {
    fn from(policies: HashMap<String, AccessPolicy>) -> Self {
        if policies.is_empty() {
            return SignedIdentifiers { items: None };
        }

        let signed_identifiers: Vec<SignedIdentifier> = policies
            .into_iter()
            .map(|(id, access_policy)| SignedIdentifier {
                id: Some(id),
                access_policy: Some(access_policy),
            })
            .collect();

        SignedIdentifiers {
            items: Some(signed_identifiers),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{blob_tags_to_string, BlobTag, BlobTags};
    use std::collections::HashMap;

    /// `BlobTags::default()` has `blob_tag_set: None` and must produce `None` so callers don't
    /// emit a stray `x-ms-tags: ` header for blobs that have no tags.
    #[test]
    fn blob_tags_to_string_returns_none_when_tag_set_is_none() {
        let tags = BlobTags::default();
        assert_eq!(blob_tags_to_string(&tags), None);
    }

    /// An explicitly empty tag vector is also "no tags". This shape is what
    /// `BlobTags::from(HashMap::new())` produces, so the result must be `None` to match the
    /// `None` case above.
    #[test]
    fn blob_tags_to_string_returns_none_when_tag_set_is_empty_vec() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![]),
        };
        assert_eq!(blob_tags_to_string(&tags), None);
    }

    /// Tag entries with a missing key OR a missing value are skipped. With the only entry
    /// incomplete, the encoded string is empty and we return `None` (same shape as "no tags").
    #[test]
    fn blob_tags_to_string_skips_entries_with_missing_key_or_value() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![
                BlobTag {
                    key: Some("orphan_key".into()),
                    value: None,
                },
                BlobTag {
                    key: None,
                    value: Some("orphan_value".into()),
                },
            ]),
        };
        assert_eq!(blob_tags_to_string(&tags), None);
    }

    /// Verifies the exact byte output for chars Azure Storage treats as separators in the
    /// `x-ms-tags` header. `&` separates tag pairs and `=` separates key from value, so both
    /// MUST be percent-encoded when they appear inside a key or value. `+` is also reserved
    /// because some URL parsers decode `+` as space.
    ///
    /// Anchored in findings.md §1.5 / Observation G.
    #[test]
    fn blob_tags_to_string_percent_encodes_reserved_chars() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![BlobTag {
                key: Some("key+name".into()),
                value: Some("v=1".into()),
            }]),
        };
        assert_eq!(
            blob_tags_to_string(&tags).as_deref(),
            Some("key%2Bname=v%3D1"),
        );
    }

    /// Verifies that `&` (the pair separator) and `=` (the key/value separator) are
    /// percent-encoded when they appear inside a key or value. Without this, a value like
    /// `c=d` in a key like `a&b` would re-parse as two pairs `a` -> (empty) and `b=c`, and a
    /// stray `=d` -- silent corruption.
    ///
    /// Anchored in findings.md Observation G. The encoder uses `NON_ALPHANUMERIC`, which is a
    /// strict superset of `{&, =}`, so this is correct by construction; this test prevents a
    /// future swap to a narrower `AsciiSet` from regressing the separator handling.
    #[test]
    fn blob_tags_to_string_percent_encodes_separators() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![BlobTag {
                key: Some("a&b".into()),
                value: Some("c=d".into()),
            }]),
        };
        assert_eq!(blob_tags_to_string(&tags).as_deref(), Some("a%26b=c%3Dd"),);
    }

    /// Verifies the exact byte output for space and `/`. Azure accepts both raw in tag values
    /// (per the allowed charset), but the wire form needs them encoded so the URL parser sees
    /// well-formed `key=value` pairs.
    #[test]
    fn blob_tags_to_string_percent_encodes_space_and_slash() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![BlobTag {
                key: Some("path with space".into()),
                value: Some("a/b".into()),
            }]),
        };
        assert_eq!(
            blob_tags_to_string(&tags).as_deref(),
            Some("path%20with%20space=a%2Fb"),
        );
    }

    /// The header format depends on `&` as the pair separator. Multiple complete tags must be
    /// joined by an unencoded `&`, in the order they appear in `blob_tag_set` (insertion order
    /// of the input `Vec<BlobTag>`). HashMap iteration order is non-deterministic, but
    /// `From<HashMap>` collects into a `Vec<BlobTag>` whose order is then preserved through
    /// this function.
    #[test]
    fn blob_tags_to_string_joins_complete_pairs_with_ampersand_in_input_order() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![
                BlobTag {
                    key: Some("a".into()),
                    value: Some("1".into()),
                },
                BlobTag {
                    key: Some("b".into()),
                    value: Some("2".into()),
                },
                BlobTag {
                    key: Some("c".into()),
                    value: Some("3".into()),
                },
            ]),
        };
        assert_eq!(blob_tags_to_string(&tags).as_deref(), Some("a=1&b=2&c=3"),);
    }

    /// Locks the invariant that `BlobTags::from(HashMap)` is a pure passthrough. If this ever
    /// starts encoding, every other test in this module breaks and the no-double-encoding
    /// claim in findings.md §3 collapses.
    #[test]
    fn from_hashmap_for_blob_tags_does_not_modify_keys_or_values() {
        let map = HashMap::from([
            ("path".to_string(), "/a/b".to_string()),
            ("with space".to_string(), "x y".to_string()),
            ("key+name".to_string(), "v=1".to_string()),
        ]);
        let tags = BlobTags::from(map.clone());
        let tag_set = tags.blob_tag_set.expect("blob_tag_set should be Some");
        let roundtripped: HashMap<String, String> = tag_set
            .into_iter()
            .map(|t| (t.key.unwrap(), t.value.unwrap()))
            .collect();
        assert_eq!(roundtripped, map);
    }

    /// Mirror of the above for the reverse direction: `From<BlobTags> for HashMap` is also a
    /// pure passthrough. Together these two tests prove the convenience conversions never
    /// touch the bytes.
    #[test]
    fn from_blob_tags_for_hashmap_does_not_modify_keys_or_values() {
        let tags = BlobTags {
            blob_tag_set: Some(vec![
                BlobTag {
                    key: Some("path".into()),
                    value: Some("/a/b".into()),
                },
                BlobTag {
                    key: Some("with space".into()),
                    value: Some("x y".into()),
                },
                BlobTag {
                    key: Some("key+name".into()),
                    value: Some("v=1".into()),
                },
            ]),
        };
        let map: HashMap<String, String> = tags.into();
        let expected = HashMap::from([
            ("path".to_string(), "/a/b".to_string()),
            ("with space".to_string(), "x y".to_string()),
            ("key+name".to_string(), "v=1".to_string()),
        ]);
        assert_eq!(map, expected);
    }
}
