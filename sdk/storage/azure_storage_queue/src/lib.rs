// Copyright (c) Microsoft Corporation. All rights reserved.
// Licensed under the MIT License.

#![doc = include_str!("../README.md")]

#[allow(unused_imports)]
mod generated;

mod logging;

use crate::models::SentMessage;
use azure_core::http::{DeserializeWith, Response, XmlFormat};
use serde::Deserialize;

impl TryFrom<Response<SentMessage, XmlFormat>> for SentMessage {
    type Error = azure_core::Error;
    fn try_from(response: Response<SentMessage, XmlFormat>) -> Result<Self, Self::Error> {
        #[derive(Clone, Default, Deserialize)]
        #[non_exhaustive]
        #[serde(rename = "QueueMessagesList")]
        struct ListOfSentMessage {
            /// The list of enqueued messages.
            #[serde(rename = "QueueMessage", skip_serializing_if = "Option::is_none")]
            pub items: Option<Vec<SentMessage>>,
        }

        let list = <ListOfSentMessage as DeserializeWith<XmlFormat>>::deserialize_with(
            response.into_body(),
        )?;
        list.items
            .unwrap_or_default()
            .into_iter()
            .next()
            .ok_or_else(|| {
                azure_core::Error::with_message(
                    azure_core::error::ErrorKind::DataConversion,
                    "No messages found in the response.",
                )
            })
    }
}

/// Data models and types used by the Azure Storage Queue service.
///
/// This module contains all the request and response models, enums, and other data types
/// used when interacting with Azure Storage Queues, including queue messages, metadata,
/// and service properties.
pub mod models {
    pub use crate::generated::models::*;
}

/// Client implementations for interacting with Azure Storage Queue service.
///
/// This module provides high-level client APIs for managing queues and queue messages,
/// including operations like creating queues, sending/receiving messages, and managing
/// queue metadata.
pub mod clients;

pub use clients::{QueueClient, QueueClientOptions, QueueServiceClient, QueueServiceClientOptions};
