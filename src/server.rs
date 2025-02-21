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

use crate::schema::original::zerocopy::{
    ClientNotification, Implementation, InitializeRequestParams, InitializeResult,
    ListPromptsResult, ListResourcesResult, ListToolsResult, RequestId, ResultData,
    ServerCapabilities, ServerCapabilitiesPrompts, ServerCapabilitiesResources,
    ServerCapabilitiesTools,
};
use crate::schema::zerocopy::{ClientMessage, ClientRequest, Message, ServerMessage, ServerResult};
use async_trait::async_trait;
use std::collections::HashMap;
use std::io;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, Stdin, Stdout};
use tokio::sync::RwLock;
use yoke::Yoke;

/// an MCP server, capable of responding to requests
pub struct MCPServer<T: Transport> {
    transport: T,
    name: String,
    version: String,
    instructions: Option<String>,
    tools: RwLock<HashMap<String, Box<dyn Tool>>>,
    resources: RwLock<HashMap<String, Box<dyn Resource>>>,
    /// Whether the client is initialized
    client_initialized: bool,
}

impl<T> MCPServer<T>
where
    T: Transport,
{
    /// Constructor
    pub fn new(
        transport: T,
        name: &str,
        version: &str,
        instructions: Option<&str>,
        tools: HashMap<String, Box<dyn Tool>>,
        resources: HashMap<String, Box<dyn Resource>>,
    ) -> Self {
        Self {
            transport,
            name: name.into(),
            version: version.into(),
            instructions: instructions.map(String::from),
            tools: RwLock::new(tools),
            resources: RwLock::new(resources),
            client_initialized: false,
        }
    }
    fn tool_add(name: &str, tool: impl Tool) {}
    fn tool_remove(name: &str) {}
    fn resource_add(name: &str, resource: impl Resource) {}
    fn resource_remove(name: &str) {}

    pub async fn run(mut self) {
        loop {
            // Receive a message from the client
            let msg = self.transport.recv().await.unwrap();
            // Parse it
            let msg: ClientMessage = serde_json::from_slice(&msg).unwrap();
            // Handle it
            use Message::*;
            dbg!(&msg);
            match msg {
                Request {
                    jsonrpc,
                    id,
                    request,
                } => {
                    use ClientRequest::*;
                    let response: ServerMessage = match request {
                        Initialize(InitializeRequestParams {
                            capabilities,
                            client_info,
                            protocol_version,
                        }) => respond_to(
                            jsonrpc,
                            id,
                            ServerResult::Initialize(InitializeResult {
                                capabilities: ServerCapabilities {
                                    experimental: Default::default(),
                                    logging: Default::default(),
                                    prompts: Some(ServerCapabilitiesPrompts {
                                        list_changed: Some(true),
                                    }),
                                    resources: Some(ServerCapabilitiesResources {
                                        list_changed: Some(true),
                                        subscribe: Some(true),
                                    }),
                                    tools: Some(ServerCapabilitiesTools {
                                        list_changed: Some(true),
                                    }),
                                },
                                instructions: match self.instructions {
                                    Some(ref instructions) => Some(instructions),
                                    None => None,
                                },
                                meta: Default::default(),
                                protocol_version,
                                server_info: Implementation {
                                    name: self.name.as_str(),
                                    version: self.version.as_str(),
                                },
                            }),
                        ),
                        Ping(_) => respond_to(
                            jsonrpc,
                            id,
                            ServerResult::Empty(ResultData {
                                meta: Default::default(),
                            }),
                        ),
                        ListResources(_) => respond_to(
                            jsonrpc,
                            id,
                            ServerResult::ListResources(ListResourcesResult {
                                meta: Default::default(),
                                next_cursor: None,
                                resources: Vec::new(),
                            }),
                        ),
                        ListResourceTemplates(_) => {
                            unimplemented!()
                        }
                        ReadResource(_) => {
                            unimplemented!()
                        }
                        Subscribe(_) => {
                            unimplemented!()
                        }
                        Unsubscribe(_) => {
                            unimplemented!()
                        }
                        ListPrompts(_) => respond_to(
                            jsonrpc,
                            id,
                            ServerResult::ListPrompts(ListPromptsResult {
                                meta: Default::default(),
                                next_cursor: None,
                                prompts: Vec::new(),
                            }),
                        ),
                        GetPrompt(_) => {
                            unimplemented!()
                        }
                        ListTools(_) => respond_to(
                            jsonrpc,
                            id,
                            ServerResult::ListTools(ListToolsResult {
                                meta: Default::default(),
                                next_cursor: None,
                                tools: Vec::new(),
                            }),
                        ),
                        CallTool(_) => {
                            unimplemented!()
                        }
                        SetLevel(_) => {
                            unimplemented!()
                        }
                        Complete(_) => {
                            unimplemented!()
                        }
                    };
                    let serialized = serde_json::to_vec(&response).unwrap();
                    dbg!(String::from_utf8_lossy(&serialized));
                    self.transport.send(&serialized).await.unwrap();
                }
                Response { .. } => {}
                Notification {
                    jsonrpc,
                    notification,
                } => {}
                Error(_) => {}
            }
        }
    }
}
pub fn respond_to<'a>(
    jsonrpc: &'a str,
    id: RequestId,
    result: ServerResult<'a>,
) -> ServerMessage<'a> {
    Message::Response {
        jsonrpc,
        id,
        result,
    }
}

impl MCPServer<StdioTransport> {
    pub fn new_stdio(
        name: &str,
        version: &str,
        instructions: Option<&str>,
        tools: HashMap<String, Box<dyn Tool>>,
        resources: HashMap<String, Box<dyn Resource>>,
    ) -> Self {
        Self::new(
            StdioTransport::new(),
            name,
            version,
            instructions,
            tools,
            resources,
        )
    }
}

#[async_trait]
pub trait Transport {
    /// Receives and stores a message from the transport
    async fn recv(&mut self) -> Result<Vec<u8>, io::Error>;
    /// Sends a messsage on the transport as bytes
    async fn send(&mut self, buf: &[u8]) -> Result<(), io::Error>;
}

/// MCP transport using stdio
pub struct StdioTransport {
    stdin: BufReader<Stdin>,
    stdout: Stdout,
}

impl StdioTransport {
    /// Constructor
    pub fn new() -> Self {
        Self {
            stdin: BufReader::new(tokio::io::stdin()),
            stdout: tokio::io::stdout(),
        }
    }
}

#[async_trait]
impl Transport for StdioTransport {
    /// Receives a message from the transport as bytes
    async fn recv(&mut self) -> Result<Vec<u8>, io::Error> {
        // Read a line from stdin
        let mut buf = Vec::new();
        self.stdin.read_until(b'\n', &mut buf).await.map(|_| buf)
    }
    /// Sends a messsage on the transport as bytes
    async fn send(&mut self, buf: &[u8]) -> Result<(), io::Error> {
        self.stdout.write_all(buf).await?;
        self.stdout.write_u8(b'\n').await?;
        self.stdout.flush().await
    }
}

pub trait Tool {}
pub trait Resource {}
