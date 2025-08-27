# claude-codes 🚀

**A tightly typed Rust interface for the Claude Code JSON protocol - unleashing the full power of Claude's capabilities in your Rust applications!**

## Why We Love Claude Code (and Why You Will Too!) 💡

Claude Code represents a paradigm shift in how we interact with AI assistants programmatically. This library brings that same revolutionary experience to Rust developers, providing **complete, type-safe access** to Claude's extensive feature set. No more wrestling with raw JSON or hoping your messages are formatted correctly - we've done the heavy lifting so you can focus on building amazing things!

### What Makes This Special?

This isn't just another API wrapper. We've meticulously reverse-engineered and documented the Claude Code protocol through extensive testing, capturing real-world message flows and ensuring every feature works exactly as expected. The result? **You get the same powerful capabilities that Claude Code uses internally**, now available in your Rust applications with zero compromises.

## ⚠️ Important Notice: Evolving with Excellence

This crate provides bindings for the **Claude Code CLI**, which is currently an **unstable and rapidly evolving interface**. But here's the exciting part: we're evolving right alongside it! Our test-driven approach means we catch protocol changes quickly and adapt, ensuring you're always working with the latest capabilities.

## 🎯 Features That Empower

### Core Capabilities
- **Type-safe message encoding/decoding** - Never worry about malformed messages again
- **JSON Lines protocol support** - Stream responses efficiently, just like Claude Code does
- **Async and sync I/O support** - Use whatever fits your application architecture
- **Comprehensive error handling** - Know exactly what went wrong and why
- **Stream processing utilities** - Handle real-time responses with ease
- **Automatic Claude CLI version compatibility checking** - Stay informed about compatibility

### Advanced Features (v0.2.0+)
- **🖼️ Image Support** - Send images directly to Claude! Support for JPEG, PNG, GIF, and WebP with automatic base64 encoding
- **🔧 Tool Use Blocks** - Full support for Claude's tool use capabilities, letting Claude interact with external systems
- **🔐 Authentication Flexibility** - OAuth tokens and API keys via environment variables
- **🧪 Integration Testing** - Battle-tested against real Claude services
- **💬 Rich Content Blocks** - Support for Text, Image, Thinking, ToolUse, and ToolResult blocks
- **🆔 UUID Session Management** - Type-safe session handling prevents common errors

### What You Can Build

With `claude-codes`, you have the power to:
- **Build intelligent CLI tools** that leverage Claude's understanding
- **Create automated workflows** with Claude as your AI assistant
- **Develop testing frameworks** that use Claude for intelligent assertions
- **Construct content pipelines** that process text and images with AI
- **Implement coding assistants** that understand context and suggest improvements
- **Design interactive applications** with streaming responses and real-time feedback

The possibilities are limited only by your imagination!

## Installation

```bash
cargo add claude-codes
```

## Usage

### Async Client

```rust
use claude_codes::AsyncClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (checks Claude CLI compatibility)
    let mut client = AsyncClient::with_defaults().await?;
    
    // Send a query and stream responses
    let mut stream = client.query_stream("What is 2 + 2?").await?;
    
    while let Some(response) = stream.next().await {
        match response {
            Ok(output) => println!("Got: {}", output.message_type()),
            Err(e) => eprintln!("Error: {}", e),
        }
    }
    
    Ok(())
}
```

### Sync Client

```rust
use claude_codes::{SyncClient, ClaudeInput};
use uuid::Uuid;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create client (checks Claude CLI compatibility)
    let mut client = SyncClient::with_defaults()?;
    
    // Send a query
    let input = ClaudeInput::user_message("What is 2 + 2?", Uuid::new_v4());
    let responses = client.query(input)?;
    
    for response in responses {
        println!("Got: {}", response.message_type());
    }
    
    Ok(())
}
```

### Working with Images 🖼️

```rust
use claude_codes::{AsyncClient, ClaudeInput};
use base64::{engine::general_purpose::STANDARD, Engine};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = AsyncClient::with_defaults().await?;
    
    // Read and encode an image
    let image_data = std::fs::read("my_diagram.png")?;
    let base64_image = STANDARD.encode(&image_data);
    
    // Send image with a question
    let input = ClaudeInput::user_message_with_image(
        base64_image,
        "image/png".to_string(),
        Some("What's shown in this diagram?".to_string()),
        uuid::Uuid::new_v4(),
    )?;
    
    // Get Claude's analysis
    client.send(&input).await?;
    // ... process responses
    
    Ok(())
}
```

### Working with Raw Protocol

```rust
use claude_codes::{Protocol, ClaudeOutput};

// Deserialize a JSON Lines message
let json_line = r#"{"type":"assistant","message":{...}}"#;
let output: ClaudeOutput = Protocol::deserialize(json_line)?;

// Serialize for sending
let serialized = Protocol::serialize(&output)?;
```

## 🌟 Real-World Applications

### Success Stories in the Making

This library is already empowering developers to build incredible things:

#### Intelligent Development Tools
Imagine a Rust analyzer that not only catches syntax errors but understands your intent. With `claude-codes`, you can build tools that review PRs, suggest optimizations, and even explain complex code patterns - all with Claude's deep understanding.

#### Automated Content Processing
Process thousands of images and documents with intelligent analysis. Whether you're building a content moderation system, a document classifier, or an accessibility tool that describes images, Claude's multimodal capabilities are now at your fingertips.

#### Interactive Learning Systems
Create educational tools that adapt to users' understanding levels. Claude can explain concepts, answer questions, and provide personalized feedback - all through your Rust application's type-safe interface.

## 💪 Why Rust + Claude = Perfect Match

### Performance Meets Intelligence
Rust's zero-cost abstractions and Claude's powerful AI create an unbeatable combination. Process millions of tokens without breaking a sweat, stream responses in real-time, and handle concurrent sessions with confidence.

### Type Safety Saves Time
Our strongly-typed interface catches errors at compile time, not runtime. No more debugging mysterious JSON parsing errors at 3 AM. The compiler is your friend, and we've made sure it knows exactly what Claude expects.

### Production-Ready Reliability
Built with the same attention to detail that makes Rust perfect for systems programming. Proper error handling, resource cleanup, and memory safety mean your Claude-powered applications can run 24/7 without surprises.

## 🚀 Getting Started is a Breeze

1. **Install Claude CLI**: `npm install -g @anthropic-ai/claude-cli`
2. **Add to your project**: `cargo add claude-codes`
3. **Start building**: Our examples and documentation guide you every step of the way

## 🤝 Join Our Community of Builders

We're not just maintaining a library - we're building a movement of developers who believe AI should be accessible, type-safe, and powerful. Every contribution, bug report, and feature request helps make this vision a reality.

### How You Can Help
- **Test with different Claude CLI versions** and report your experience
- **Share your use cases** - what amazing things are you building?
- **Contribute code** - we welcome PRs that improve compatibility and features
- **Spread the word** - help other Rust developers discover these capabilities

### Compatibility Reporting

- **Current tested version**: Claude CLI 1.0.89
- **Compatibility reports needed**: If you're using this crate with a different version of Claude CLI (whether it works or fails), please report your experience at:
  
  **https://github.com/meawoppl/rust-claude-codes/pulls**

When creating a compatibility report, please include:
- Your Claude CLI version (run `claude --version`)
- Whether the crate worked correctly or what errors you encountered
- Any message types that failed to deserialize

The crate will automatically warn you if you're using a newer Claude CLI version than what has been tested. You can work around version checks if needed (see documentation), but please report your results to help the community!

## 🎯 Our Mission

We believe that AI should amplify human capability, not replace it. By providing Rust developers with first-class access to Claude's capabilities, we're enabling a future where AI and systems programming work hand-in-hand to solve problems we couldn't tackle alone.

Every line of code in this library represents our commitment to that vision: **making AI accessible, type-safe, and incredibly powerful** for the Rust community.

## 📈 The Road Ahead

This is just the beginning. As Claude's capabilities expand and the protocol evolves, we'll be right there - documenting, testing, and providing rock-solid Rust bindings for every new feature. Together, we're not just keeping up with the AI revolution - we're helping to shape it.

### Coming Soon
- Enhanced tool use patterns and examples
- Performance benchmarks and optimization guides
- Extended documentation with real-world case studies
- Community-contributed examples and patterns

## License

Licensed under the Apache License, Version 2.0 ([LICENSE](LICENSE) or http://www.apache.org/licenses/LICENSE-2.0).

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed under the Apache-2.0 license,
without any additional terms or conditions.

---

**Built with ❤️ by developers who believe in the power of type-safe AI**

*Special thanks to the Claude team for creating such an incredible AI assistant, and to the Rust community for being awesome.*
