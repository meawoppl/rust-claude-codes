//! Declares a client-side tool and answers the harness when the model calls it.
//!
//! ```sh
//! export GEMINI_API_KEY=...
//! export ANTIGRAVITY_HARNESS_PATH=/path/to/localharness
//! cargo run -p antigravity-codes --example custom_tool
//! ```
//!
//! Two halves have to agree for this to work:
//!
//! 1. [`HarnessOptions::tool`] *declares* the tool, so the harness advertises
//!    it to the model along with its JSON Schema.
//! 2. [`Handlers::tool`] *implements* it, keyed by the same name.
//!
//! Declare without implementing and the call comes back as a tool error;
//! implement without declaring and the model never calls it.

use antigravity_codes::handlers::Handlers;
use antigravity_codes::protocol::{LifecycleHook, Tool, ToolResponse};
use antigravity_codes::{Client, HarnessOptions, ModelBuilder, Result};

const SCHEMA: &str = r#"{
  "type": "object",
  "properties": {
    "city": {"type": "string", "description": "City to look up"}
  },
  "required": ["city"]
}"#;

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let options = HarnessOptions::new()
        .workspace(std::env::current_dir()?)
        .model(ModelBuilder::gemini(
            "gemini-3-pro-preview",
            std::env::var("GEMINI_API_KEY").unwrap_or_default(),
        ))
        .tool(Tool {
            name: Some("get_weather".into()),
            description: Some("Returns the current weather for a city.".into()),
            parameters_json_schema: Some(SCHEMA.into()),
            ..Default::default()
        })
        .hook(LifecycleHook::PreTool);

    let handlers = Handlers::new()
        .tool("get_weather", |call| async move {
            // `arguments_json` is the model's arguments, already validated
            // against the schema declared above.
            let args: serde_json::Value =
                serde_json::from_str(call.arguments_json.as_deref().unwrap_or("{}"))
                    .unwrap_or_default();
            let city = args
                .get("city")
                .and_then(|c| c.as_str())
                .unwrap_or("nowhere");
            println!("[tool] get_weather({city})");

            ToolResponse::ok(
                call.id.unwrap_or_default(),
                serde_json::json!({ "city": city, "conditions": "clear", "celsius": 21 })
                    .to_string(),
            )
        })
        .on_hook(|request| async move {
            println!("[hook] {:?} {:?}", request.r#type, request.name);
            // An empty result means "no opinion" — the turn proceeds.
            antigravity_codes::protocol::CallHookResponse {
                request_id: request.request_id,
                empty_result: Some(Default::default()),
                ..Default::default()
            }
        });

    let mut client = Client::launch_with(options, handlers).await?;
    let mut turn = client
        .send("What is the weather in Reykjavik? Use your tool.")
        .await?;

    while let Some(step) = turn.next_step().await? {
        if let Some(text) = step.user_facing_text() {
            println!("{text}");
        }
    }

    client.shutdown().await
}
