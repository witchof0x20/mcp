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
/// High-level representations and schemas for the Model Context Protocol
pub mod schema;
/// Derive macro for Tool queries
pub use tool_macros;
/// Server component
#[cfg(feature = "server")]
pub mod server;
