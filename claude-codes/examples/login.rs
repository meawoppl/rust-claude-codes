//! Interactive walkthrough of the `auth` feature's login tooling.
//!
//! ```bash
//! cargo run -p claude-codes --features auth --example login            # setup-token
//! cargo run -p claude-codes --features auth --example login claudeai   # subscription login
//! cargo run -p claude-codes --features auth --example login console    # console login
//! ```

use claude_codes::auth::{auth_status, LoginFlow, LoginMode};
use std::io::{BufRead, Write};
use std::time::Duration;

fn main() -> claude_codes::Result<()> {
    let mode = match std::env::args().nth(1).as_deref() {
        None | Some("setup-token") => LoginMode::SetupToken,
        Some("claudeai") => LoginMode::ClaudeAi,
        Some("console") => LoginMode::Console,
        Some(other) => {
            eprintln!("unknown mode {other:?}; use setup-token | claudeai | console");
            std::process::exit(2);
        }
    };

    println!("current status: {:?}\n", auth_status()?);

    let mut flow = LoginFlow::start(mode)?;
    let url = flow.auth_url(Duration::from_secs(60))?;
    println!("Visit this URL to sign in:\n\n  {url}\n");

    print!("Paste the code shown after authorizing: ");
    std::io::stdout().flush()?;
    let mut code = String::new();
    std::io::stdin().lock().read_line(&mut code)?;
    flow.submit_code(&code)?;

    let outcome = flow.finish(Duration::from_secs(120))?;
    match &outcome.token {
        Some(token) => println!(
            "Minted long-lived token (via {:?}): {token}",
            outcome.token_source
        ),
        None => println!(
            "Login completed (credentials_updated: {}); status now: {:?}",
            outcome.credentials_updated,
            auth_status()?
        ),
    }
    Ok(())
}
