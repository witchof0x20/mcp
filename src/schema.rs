// Rust MCP
// Copyright (C) 2025 Jade Harley
//
// This program is free software: you can redistribute it and/or modify it
// under the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version.
//
// This program is distributed in the hope that it will be useful, but WITHOUT
// ANY WARRANTY; without even the implied warranty of  MERCHANTABILITY or
// FITNESS FOR A PARTICULAR PURPOSE. See the GNU General Public License for
// more details.
//
// You should have received a copy of the GNU General Public License along with
// this program.  If not, see <http://www.gnu.org/licenses/>.
use serde::{Deserialize, Serialize};
use serde_valid::Validate;

/// MCP Protocol version
pub const VERSION: &str = "2024-11-05";

/// Encapsulates anything that will be sent from a particular side
#[derive(Debug, Deserialize, Serialize, Validate)]
#[serde(untagged)]
pub enum Message<RQ, RS, N> {
    /// JSONRPC Request
    Request {
        #[validate(custom = validate_jsonrpc_version)]
        #[doc(hidden)]
        jsonrpc: String,
        id: original::zerocopy::RequestId,
        #[serde(flatten)]
        request: RQ,
    },
    /// JSONRPC Notification
    Notification {
        #[validate(custom = validate_jsonrpc_version)]
        #[doc(hidden)]
        jsonrpc: String,
        #[serde(flatten)]
        notification: N,
    },
    /// JSONRPC Error
    Error(#[validate(custom = validate_jsonrpc_error)] original::JsonrpcError),
    /// JSONRPC Response
    Response {
        #[validate(custom = validate_jsonrpc_version)]
        #[doc(hidden)]
        jsonrpc: String,
        id: original::RequestId,
        result: RS,
    },
}

/// A message sent by an MCP client
pub type ClientMessage = Message<ClientRequest, ClientResult, ClientNotification>;
/// A message sent by an MCP server
pub type ServerMessage = Message<ServerRequest, ServerResult, ServerNotification>;

/// Custom serde validation function to make sure jsonrpc is the correct version
fn validate_jsonrpc_version(val: &str) -> Result<(), serde_valid::validation::Error> {
    if val == "2.0" {
        Ok(())
    } else {
        Err(serde_valid::validation::Error::Custom(
            "JSONRPC version must be 2.0".into(),
        ))
    }
}

/// Custom serde validation function to make sure jsonrpc is the correct version
fn validate_jsonrpc_error(
    err: &original::JsonrpcError,
) -> Result<(), serde_valid::validation::Error> {
    validate_jsonrpc_version(&err.jsonrpc)
}

/// Request made by the client
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ClientRequest {
    #[serde(rename = "initialize")]
    Initialize(original::InitializeRequestParams),
    #[serde(rename = "ping")]
    Ping(original::PingRequestParams),
    #[serde(rename = "resources/list")]
    ListResources(original::ListResourcesRequestParams),
    #[serde(rename = "resources/templates/list")]
    ListResourceTemplates(original::ListResourceTemplatesRequestParams),
    #[serde(rename = "resources/read")]
    ReadResource(original::ReadResourceRequestParams),
    #[serde(rename = "resources/subscribe")]
    Subscribe(original::SubscribeRequestParams),
    #[serde(rename = "resources/unsubscribe")]
    Unsubscribe(original::UnsubscribeRequestParams),
    #[serde(rename = "prompts/list")]
    ListPrompts(original::ListPromptsRequestParams),
    #[serde(rename = "prompts/get")]
    GetPrompt(original::GetPromptRequestParams),
    #[serde(rename = "tools/list")]
    ListTools(original::ListToolsRequestParams),
    #[serde(rename = "tools/call")]
    CallTool(original::CallToolRequestParams),
    #[serde(rename = "logging/setlevel")]
    SetLevel(original::SetLevelRequestParams),
    #[serde(rename = "completion/complete")]
    Complete(original::CompleteRequestParams),
}

/// Result sent by the client
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ClientResult {
    Result(original::ResultData),
    CreateMessage(original::CreateMessageResult),
    ListRoots(original::ListRootsResult),
}

/// Notification sent by the client
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ClientNotification {
    #[serde(rename = "notifications/cancelled")]
    Cancelled(original::CancelledNotificationParams),
    #[serde(rename = "notifications/initialized")]
    Initialized(original::InitializedNotificationParams),
    #[serde(rename = "notifications/progress")]
    Progress(original::ProgressNotificationParams),
    #[serde(rename = "notifications/roots/list_changed")]
    RootsListChanged(original::RootsListChangedNotificationParams),
}

/// Request made by the server
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ServerRequest {
    #[serde(rename = "ping")]
    Ping(original::PingRequestParams),
    #[serde(rename = "sampling/createMessage")]
    CreateMessage(original::CreateMessageRequestParams),
    #[serde(rename = "roots/list")]
    ListRoots(original::ListRootsRequestParams),
}
/// Result sent by the server
#[derive(Debug, Deserialize, Serialize)]
#[serde(untagged)]
pub enum ServerResult {
    Empty(original::ResultData),
    Initialize(original::InitializeResult),
    ListResources(original::ListResourcesResult),
    ListResourceTemplates(original::ListResourceTemplatesResult),
    ReadResource(original::ReadResourceResult),
    ListPrompts(original::ListPromptsResult),
    GetPrompt(original::GetPromptResult),
    ListTools(original::ListToolsResult),
    CallTool(original::CallToolResult),
    Complete(original::CompleteResult),
}

/// Notification sent by the server
#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "method", content = "params")]
pub enum ServerNotification {
    #[serde(rename = "notifications/cancelled")]
    Cancelled(original::CancelledNotificationParams),
    #[serde(rename = "notifications/progress")]
    Progress(original::ProgressNotificationParams),
    #[serde(rename = "notifications/resources/list_changed")]
    ResourceListChanged(original::ResourceListChangedNotificationParams),
    #[serde(rename = "notifications/resources/updated")]
    ResourceUpdated(original::ResourceUpdatedNotificationParams),
    #[serde(rename = "notifications/prompts/list_changed")]
    PromptListChanged(original::PromptListChangedNotificationParams),
    #[serde(rename = "notifications/tools/list_changed")]
    ToolListChanged(original::ToolListChangedNotificationParams),
    #[serde(rename = "notifications/message")]
    LoggingMessage(original::LoggingMessageNotificationParams),
}
/// Zero-copy versions of high-level MCP schema
pub mod zerocopy {
    use super::{original::zerocopy as original, validate_jsonrpc_version};
    use serde::{Deserialize, Serialize};
    use serde_valid::Validate;

    /// Encapsulates anything that will be sent from a particular side
    #[derive(Debug, Deserialize, Serialize, Validate)]
    #[serde(untagged)]
    pub enum Message<'a, RQ, RS, N> {
        /// JSONRPC Request
        Request {
            #[validate(custom = validate_jsonrpc_version)]
            #[doc(hidden)]
            jsonrpc: &'a str,
            id: original::RequestId,
            #[serde(flatten)]
            request: RQ,
        },
        /// JSONRPC Notification
        Notification {
            #[validate(custom = validate_jsonrpc_version)]
            #[doc(hidden)]
            jsonrpc: &'a str,
            #[serde(flatten)]
            notification: N,
        },
        /// JSONRPC Error
        Error(
            #[serde(borrow)]
            #[validate(custom = validate_jsonrpc_error)]
            original::JsonrpcError<'a>,
        ),
        /// JSONRPC Response
        Response {
            #[validate(custom = validate_jsonrpc_version)]
            #[doc(hidden)]
            jsonrpc: &'a str,
            id: original::RequestId,
            result: RS,
        },
    }

    /// A message sent by an MCP client
    pub type ClientMessage<'a> =
        Message<'a, ClientRequest<'a>, ClientResult<'a>, ClientNotification<'a>>;
    /// A message sent by an MCP server
    pub type ServerMessage<'a> =
        Message<'a, ServerRequest<'a>, ServerResult<'a>, ServerNotification<'a>>;

    /// Custom serde validation function to make sure jsonrpc is the correct version
    fn validate_jsonrpc_error<'a>(
        err: &original::JsonrpcError<'a>,
    ) -> Result<(), serde_valid::validation::Error> {
        validate_jsonrpc_version(err.jsonrpc)
    }

    /// Request made by the client
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "method", content = "params")]
    pub enum ClientRequest<'a> {
        #[serde(rename = "initialize")]
        Initialize(#[serde(borrow)] original::InitializeRequestParams<'a>),
        #[serde(rename = "ping")]
        Ping(original::PingRequestParams),
        #[serde(rename = "resources/list")]
        ListResources(#[serde(borrow)] original::ListResourcesRequestParams<'a>),
        #[serde(rename = "resources/templates/list")]
        ListResourceTemplates(#[serde(borrow)] original::ListResourceTemplatesRequestParams<'a>),
        #[serde(rename = "resources/read")]
        ReadResource(#[serde(borrow)] original::ReadResourceRequestParams<'a>),
        #[serde(rename = "resources/subscribe")]
        Subscribe(#[serde(borrow)] original::SubscribeRequestParams<'a>),
        #[serde(rename = "resources/unsubscribe")]
        Unsubscribe(#[serde(borrow)] original::UnsubscribeRequestParams<'a>),
        #[serde(rename = "prompts/list")]
        ListPrompts(#[serde(borrow)] original::ListPromptsRequestParams<'a>),
        #[serde(rename = "prompts/get")]
        GetPrompt(#[serde(borrow)] original::GetPromptRequestParams<'a>),
        #[serde(rename = "tools/list")]
        ListTools(#[serde(borrow)] original::ListToolsRequestParams<'a>),
        #[serde(rename = "tools/call")]
        CallTool(#[serde(borrow)] original::CallToolRequestParams<'a>),
        #[serde(rename = "logging/setlevel")]
        SetLevel(original::SetLevelRequestParams),
        #[serde(rename = "completion/complete")]
        Complete(#[serde(borrow)] original::CompleteRequestParams<'a>),
    }

    /// Result sent by the client
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum ClientResult<'a> {
        Result(original::ResultData),
        CreateMessage(#[serde(borrow)] original::CreateMessageResult<'a>),
        ListRoots(#[serde(borrow)] original::ListRootsResult<'a>),
    }

    /// Notification sent by the client
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "method", content = "params")]
    pub enum ClientNotification<'a> {
        #[serde(rename = "notifications/cancelled")]
        Cancelled(#[serde(borrow)] original::CancelledNotificationParams<'a>),
        #[serde(rename = "notifications/initialized")]
        Initialized(original::InitializedNotificationParams),
        #[serde(rename = "notifications/progress")]
        Progress(original::ProgressNotificationParams),
        #[serde(rename = "notifications/roots/list_changed")]
        RootsListChanged(original::RootsListChangedNotificationParams),
    }

    /// Request made by the server
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "method", content = "params")]
    pub enum ServerRequest<'a> {
        #[serde(rename = "ping")]
        Ping(original::PingRequestParams),
        #[serde(rename = "sampling/createMessage")]
        CreateMessage(#[serde(borrow)] original::CreateMessageRequestParams<'a>),
        #[serde(rename = "roots/list")]
        ListRoots(original::ListRootsRequestParams),
    }
    /// Result sent by the server
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(untagged)]
    pub enum ServerResult<'a> {
        Empty(original::ResultData),
        Initialize(#[serde(borrow)] original::InitializeResult<'a>),
        ListResources(#[serde(borrow)] original::ListResourcesResult<'a>),
        ListResourceTemplates(#[serde(borrow)] original::ListResourceTemplatesResult<'a>),
        ReadResource(#[serde(borrow)] original::ReadResourceResult<'a>),
        ListPrompts(#[serde(borrow)] original::ListPromptsResult<'a>),
        GetPrompt(#[serde(borrow)] original::GetPromptResult<'a>),
        ListTools(#[serde(borrow)] original::ListToolsResult<'a>),
        CallTool(#[serde(borrow)] original::CallToolResult<'a>),
        Complete(#[serde(borrow)] original::CompleteResult<'a>),
    }

    /// Notification sent by the server
    #[derive(Debug, Deserialize, Serialize)]
    #[serde(tag = "method", content = "params")]
    pub enum ServerNotification<'a> {
        #[serde(rename = "notifications/cancelled")]
        Cancelled(#[serde(borrow)] original::CancelledNotificationParams<'a>),
        #[serde(rename = "notifications/progress")]
        Progress(original::ProgressNotificationParams),
        #[serde(rename = "notifications/resources/list_changed")]
        ResourceListChanged(original::ResourceListChangedNotificationParams),
        #[serde(rename = "notifications/resources/updated")]
        ResourceUpdated(#[serde(borrow)] original::ResourceUpdatedNotificationParams<'a>),
        #[serde(rename = "notifications/prompts/list_changed")]
        PromptListChanged(original::PromptListChangedNotificationParams),
        #[serde(rename = "notifications/tools/list_changed")]
        ToolListChanged(original::ToolListChangedNotificationParams),
        #[serde(rename = "notifications/message")]
        LoggingMessage(#[serde(borrow)] original::LoggingMessageNotificationParams<'a>),
    }
}

/// MCP Schemas imported and converted from the official MCP specification
pub mod original {
    include!(concat!(env!("OUT_DIR"), "/schema.rs"));
}
