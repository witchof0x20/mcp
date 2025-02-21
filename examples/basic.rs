use mcp::server::{MCPServer, StdioTransport};

#[tokio::main]
async fn main() {
    let transport = StdioTransport::new();
    let mut server = MCPServer::new(
        transport,
        "basic",
        "0.1",
        Some("A test MCP server"),
        Default::default(),
        Default::default(),
    );
    server.run().await;
}
