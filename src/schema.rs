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
use yoke::{Yoke, Yokeable};

/// Deserializes a JSON byte vector into a Yoke-wrapped instance of type T
///
/// # Type Parameters
/// * `T` - The target type for deserialization
///
/// # Arguments
/// * `data` - JSON data as a byte vector
///
/// # Returns
/// * `Result<Yoke<T, Vec<u8>>, serde_json::Error>` - Either a Yoke-wrapped T or a deserialization error
pub fn json_from_vec<T>(data: Vec<u8>) -> Result<Yoke<T, Vec<u8>>, serde_json::Error>
where
    T: for<'a> Yokeable<'a>,
    for<'a> <T as Yokeable<'a>>::Output: Deserialize<'a>,
{
    Yoke::try_attach_to_cart(data, |contents| serde_json::from_slice(contents))
}

/// MCP Protocol version
pub const VERSION: &str = "2024-11-05";

/// Encapsulates anything that will be sent from a particular side
#[derive(Deserialize, Serialize, Validate, Yokeable)]
#[serde(untagged)]
pub enum Message<'a, RQ, RS, N> {
    /// JSONRPC Request
    Request(#[serde(borrow)] JSONRPCRequest<'a, RQ>),
    /// JSONRPC Response
    Response(#[serde(borrow)] JSONRPCResponse<'a, RS>),
    /// JSONRPC Notification
    Notification(#[serde(borrow)] JSONRPCNotification<'a, N>),
    /// JSONRPC Error
    Error(
        #[serde(borrow)]
        #[validate(custom = validate_jsonrpc_error)]
        original::JsonrpcError<'a>,
    ),
}

/// A message sent by an MCP client
pub type ClientMessage<'a> =
    Message<'a, ClientRequest<'a>, ClientResult<'a>, ClientNotification<'a>>;
/// A message sent by an MCP server
pub type ServerMessage<'a> =
    Message<'a, ServerRequest<'a>, ServerResult<'a>, ServerNotification<'a>>;

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
fn validate_jsonrpc_error<'a>(
    err: &original::JsonrpcError<'a>,
) -> Result<(), serde_valid::validation::Error> {
    validate_jsonrpc_version(err.jsonrpc)
}

/// A JSONRPC Request
#[derive(Debug, Deserialize, Serialize, Validate, Yokeable)]
pub struct JSONRPCRequest<'a, R> {
    #[validate(custom = validate_jsonrpc_version)]
    jsonrpc: &'a str,
    pub id: original::RequestId,
    pub params: original::JsonrpcRequestParams,
    #[serde(flatten)]
    pub request: R,
}

/// Metadata structure containing progress tracking information
#[derive(Debug, Deserialize, Serialize, Yokeable)]
pub struct Metadata<'a> {
    /// An opaque token used for tracking progress notifications.
    /// When specified, indicates the caller is requesting out-of-band
    /// progress notifications. The receiver is not obligated to provide
    /// these notifications.
    pub progress_token: Option<&'a str>,
}

/// A JSONRPC response
#[derive(Debug, Deserialize, Serialize, Validate, Yokeable)]
pub struct JSONRPCResponse<'a, R> {
    #[validate(custom = validate_jsonrpc_version)]
    jsonrpc: &'a str,
    id: original::RequestId,
    #[serde(flatten)]
    pub result: R,
}
/// A JSONRPC notification
#[derive(Debug, Deserialize, Serialize, Validate, Yokeable)]
pub struct JSONRPCNotification<'a, N> {
    #[validate(custom = validate_jsonrpc_version)]
    jsonrpc: &'a str,
    #[serde(flatten)]
    pub notification: N,
}

/// Request made by the client
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(tag = "method")]
pub enum ClientRequest<'a> {
    #[serde(rename = "initialize")]
    InitializeRequest(#[serde(borrow)] original::InitializeRequestParams<'a>),
    #[serde(rename = "ping")]
    PingRequest(original::PingRequestParams),
    #[serde(rename = "resources/list")]
    ListResourcesRequest(#[serde(borrow)] original::ListResourcesRequestParams<'a>),
    #[serde(rename = "resources/templates/list")]
    ListResourceTemplatesRequest(#[serde(borrow)] original::ListResourceTemplatesRequestParams<'a>),
    #[serde(rename = "resources/read")]
    ReadResourceRequest(#[serde(borrow)] original::ReadResourceRequestParams<'a>),
    #[serde(rename = "resources/subscribe")]
    SubscribeRequest(#[serde(borrow)] original::SubscribeRequestParams<'a>),
    #[serde(rename = "resources/unsubscribe")]
    UnsubscribeRequest(#[serde(borrow)] original::UnsubscribeRequestParams<'a>),
    #[serde(rename = "prompts/list")]
    ListPromptsRequest(#[serde(borrow)] original::ListPromptsRequestParams<'a>),
    #[serde(rename = "prompts/get")]
    GetPromptRequest(#[serde(borrow)] original::GetPromptRequestParams<'a>),
    #[serde(rename = "tools/list")]
    ListToolsRequest(#[serde(borrow)] original::ListToolsRequestParams<'a>),
    #[serde(rename = "tools/call")]
    CallToolRequest(#[serde(borrow)] original::CallToolRequestParams<'a>),
    #[serde(rename = "logging/setlevel")]
    SetLevelRequest(original::SetLevelRequestParams),
    #[serde(rename = "completion/complete")]
    CompleteRequest(#[serde(borrow)] original::CompleteRequestParams<'a>),
}

/// Result sent by the client
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(untagged)]
pub enum ClientResult<'a> {
    Result(original::ResultData),
    CreateMessageResult(#[serde(borrow)] original::CreateMessageResult<'a>),
    ListRootsResult(#[serde(borrow)] original::ListRootsResult<'a>),
}

/// Notification sent by the client
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(tag = "method")]
pub enum ClientNotification<'a> {
    #[serde(rename = "notifications/cancelled")]
    CancelledNotification(#[serde(borrow)] original::CancelledNotificationParams<'a>),
    #[serde(rename = "notifications/initialized")]
    InitializedNotification(original::InitializedNotificationParams),
    #[serde(rename = "notifications/progress")]
    ProgressNotification(original::ProgressNotificationParams),
    #[serde(rename = "notifications/roots/list_changed")]
    RootsListChangedNotification(original::RootsListChangedNotificationParams),
}

/// Request made by the server
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(tag = "method")]
pub enum ServerRequest<'a> {
    #[serde(rename = "ping")]
    PingRequest(original::PingRequestParams),
    #[serde(rename = "sampling/createMessage")]
    CreateMessageRequest(#[serde(borrow)] original::CreateMessageRequestParams<'a>),
    #[serde(rename = "roots/list")]
    ListRootsRequest(original::ListRootsRequestParams),
}
/// Result sent by the server
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(untagged)]
pub enum ServerResult<'a> {
    Result(original::ResultData),
    InitializeResult(#[serde(borrow)] original::InitializeResult<'a>),
    ListResourcesResult(#[serde(borrow)] original::ListResourcesResult<'a>),
    ListResourceTemplatesResult(#[serde(borrow)] original::ListResourceTemplatesResult<'a>),
    ReadResourceResult(#[serde(borrow)] original::ReadResourceResult<'a>),
    ListPromptsResult(#[serde(borrow)] original::ListPromptsResult<'a>),
    GetPromptResult(#[serde(borrow)] original::GetPromptResult<'a>),
    ListToolsResult(#[serde(borrow)] original::ListToolsResult<'a>),
    CallToolResult(#[serde(borrow)] original::CallToolResult<'a>),
    CompleteResult(#[serde(borrow)] original::CompleteResult<'a>),
}

/// Notification sent by the server
#[derive(Deserialize, Serialize, Yokeable)]
#[serde(tag = "method")]
pub enum ServerNotification<'a> {
    #[serde(rename = "notifications/cancelled")]
    CancelledNotification(#[serde(borrow)] original::CancelledNotificationParams<'a>),
    #[serde(rename = "notifications/progress")]
    ProgressNotification(original::ProgressNotificationParams),
    #[serde(rename = "notifications/resources/list_changed")]
    ResourceListChangedNotification(original::ResourceListChangedNotificationParams),
    #[serde(rename = "notifications/resources/updated")]
    ResourceUpdatedNotification(#[serde(borrow)] original::ResourceUpdatedNotificationParams<'a>),
    #[serde(rename = "notifications/prompts/list_changed")]
    PromptListChangedNotification(original::PromptListChangedNotificationParams),
    #[serde(rename = "notifications/tools/list_changed")]
    ToolListChangedNotification(original::ToolListChangedNotificationParams),
    #[serde(rename = "notifications/message")]
    LoggingMessageNotification(#[serde(borrow)] original::LoggingMessageNotificationParams<'a>),
}

/// MCP Schemas imported and converted from the official MCP specification
pub mod original {
    include!(concat!(env!("OUT_DIR"), "/schema.rs"));
}
