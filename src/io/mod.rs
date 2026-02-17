//! Core message types for Claude communication.
//!
//! This module defines the primary message structures used in the Claude protocol:
//!
//! - [`ClaudeInput`] - Messages sent to Claude
//! - [`ClaudeOutput`] - Messages received from Claude
//! - [`ContentBlock`] - Different types of content within messages
//!
//! # Message Flow
//!
//! 1. Create a [`ClaudeInput`] with your query
//! 2. Send it to Claude via a client
//! 3. Receive [`ClaudeOutput`] messages in response
//! 4. Handle different output types (System, Assistant, Result)
//!
//! # Example
//!
//! ```
//! use claude_codes::{ClaudeInput, ClaudeOutput};
//!
//! // Create an input message
//! let input = ClaudeInput::user_message("Hello, Claude!", uuid::Uuid::new_v4());
//!
//! // Parse an output message
//! let json = r#"{"type":"assistant","message":{"role":"assistant","content":[]}}"#;
//! match ClaudeOutput::parse_json(json) {
//!     Ok(output) => println!("Got: {}", output.message_type()),
//!     Err(e) => eprintln!("Parse error: {}", e),
//! }
//! ```

mod claude_input;
mod claude_output;
mod content_blocks;
mod control;
mod errors;
mod mcp;
mod message_types;
mod result;

pub use claude_input::*;
pub use claude_output::*;
pub use content_blocks::*;
pub use control::*;
pub use errors::*;
pub use mcp::*;
pub use message_types::*;
pub use result::*;
