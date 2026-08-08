//! Streams one turn of a conversation.
//!
//! ```sh
//! export GEMINI_API_KEY=...
//! export ANTIGRAVITY_HARNESS_PATH=/path/to/localharness
//! cargo run -p antigravity-codes --example stream_chat -- "what files are here?"
//! ```

use std::io::Write;

use antigravity_codes::{Client, HarnessOptions, ModelBuilder, Result, StepKind};

#[tokio::main]
async fn main() -> Result<()> {
    env_logger::init();

    let prompt = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "Say hello in five words.".into());
    let api_key = std::env::var("GEMINI_API_KEY").unwrap_or_else(|_| {
        eprintln!("GEMINI_API_KEY is not set; the harness will reject the model call");
        String::new()
    });
    let workspace = std::env::current_dir()?;

    let mut client = Client::launch(
        HarnessOptions::new()
            .workspace(&workspace)
            .model(ModelBuilder::gemini("gemini-3-pro-preview", api_key)),
    )
    .await?;

    println!("conversation {}", client.cascade_id().unwrap_or("<none>"));

    let mut turn = client.send(prompt).await?;
    let mut printed = 0usize;
    let result = loop {
        match turn.next_step().await {
            Ok(Some(step)) => {
                if step.kind != StepKind::Message {
                    println!(
                        "\n[{:?}] {}",
                        step.kind,
                        step.update.step_index.unwrap_or(0)
                    );
                    continue;
                }
                // Print only what is new, so re-sent steps do not duplicate.
                if let Some(text) = step.user_facing_text() {
                    if text.len() > printed {
                        print!("{}", &text[printed..]);
                        let _ = std::io::stdout().flush();
                        printed = text.len();
                    }
                }
            }
            Ok(None) => break Ok(()),
            Err(e) => break Err(e),
        }
    };
    println!();

    if let Some(usage) = client.usage() {
        println!("tokens: {:?} total", usage.total_token_count);
    }

    client.shutdown().await?;
    result
}
