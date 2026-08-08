//! Best-effort local auth status read — **private-contract territory**.
//!
//! The sanctioned way to read account state is protocol-native
//! ([`AsyncClient::account_read`](crate::client_async::AsyncClient::account_read)),
//! which requires standing up an app-server connection. For cheap status
//! probes (dashboards, launcher matrices) this module reads what the codex
//! CLI itself persists at `$CODEX_HOME/auth.json` (default
//! `~/.codex/auth.json`) and decodes the display-only identity claims from
//! the stored `id_token`.
//!
//! **Caveats, deliberately loud:**
//!
//! - `auth.json` is codex's internal storage, not a published interface.
//!   Its layout can change in any CLI release. This crate owns that risk so
//!   consumers don't have to: the shape is unit-tested against a captured
//!   fixture and exercised against the real file by the live integration
//!   suite, so a layout change becomes a crate patch, not a silent
//!   downstream break.
//! - The JWT payload is base64-decoded **without signature verification** —
//!   the values come from a file the user's own CLI wrote, and they are fit
//!   for display labels only. Never use them for authorization decisions.
//! - A `logged_in: true` here means "credentials are stored", not "they
//!   still work" — tokens expire and get refreshed by the CLI. For a
//!   liveness answer use `account_read` or `codex login status`.

use crate::error::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::path::PathBuf;

/// Serde-shaped local auth snapshot, fit for relaying to UIs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LocalAuthStatus {
    /// Credentials are present on disk (see module docs: stored ≠ live).
    pub logged_in: bool,
    /// `auth_mode` as stored, e.g. `"chatgpt"` or `"apikey"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth_mode: Option<String>,
    /// `email` claim from the stored id_token (display only).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    /// `chatgpt_plan_type` from the id_token's OpenAI auth claim
    /// (display only), e.g. `"plus"`, `"pro"`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub plan_type: Option<String>,
    /// `tokens.account_id` as stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    /// `last_refresh` timestamp string as stored.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,
}

/// `$CODEX_HOME/auth.json`, defaulting to `~/.codex/auth.json`.
pub fn auth_json_path() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("CODEX_HOME") {
        return Some(PathBuf::from(home).join("auth.json"));
    }
    let home = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE"))?;
    Some(PathBuf::from(home).join(".codex/auth.json"))
}

/// Read the local auth snapshot from the default location.
///
/// A missing file is `Ok` with `logged_in: false` (that's what logged-out
/// looks like); an unreadable or unparseable file is an error.
pub fn auth_status_local() -> Result<LocalAuthStatus> {
    let Some(path) = auth_json_path() else {
        return Err(Error::Protocol(
            "no home directory to resolve auth.json against".to_string(),
        ));
    };
    if !path.exists() {
        return Ok(LocalAuthStatus {
            logged_in: false,
            auth_mode: None,
            email: None,
            plan_type: None,
            account_id: None,
            last_refresh: None,
        });
    }
    auth_status_from_json(&std::fs::read_to_string(&path)?)
}

/// Parse a snapshot out of `auth.json` contents.
pub fn auth_status_from_json(raw: &str) -> Result<LocalAuthStatus> {
    let v: Value = serde_json::from_str(raw)?;
    let auth_mode = v
        .get("auth_mode")
        .and_then(Value::as_str)
        .map(str::to_string);
    let api_key_present = v
        .get("OPENAI_API_KEY")
        .and_then(Value::as_str)
        .is_some_and(|k| !k.trim().is_empty());
    let tokens = v.get("tokens").filter(|t| t.is_object());
    let account_id = tokens
        .and_then(|t| t.get("account_id"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let last_refresh = v
        .get("last_refresh")
        .and_then(Value::as_str)
        .map(str::to_string);

    let mut email = None;
    let mut plan_type = None;
    if let Some(id_token) = tokens
        .and_then(|t| t.get("id_token"))
        .and_then(Value::as_str)
    {
        if let Some(claims) = decode_jwt_claims(id_token) {
            email = claims
                .get("email")
                .and_then(Value::as_str)
                .map(str::to_string);
            plan_type = claims
                .get("https://api.openai.com/auth")
                .and_then(|a| a.get("chatgpt_plan_type"))
                .and_then(Value::as_str)
                .map(str::to_string);
        }
    }

    Ok(LocalAuthStatus {
        logged_in: tokens.is_some() || api_key_present,
        auth_mode,
        email,
        plan_type,
        account_id,
        last_refresh,
    })
}

/// Decode a JWT's payload segment (base64url, unverified) into JSON.
fn decode_jwt_claims(jwt: &str) -> Option<Value> {
    let payload = jwt.split('.').nth(1)?;
    let bytes = base64url_decode(payload)?;
    serde_json::from_slice(&bytes).ok()
}

/// Minimal base64url (no padding) decoder — display-only path, so a tiny
/// hand-rolled table beats a dependency.
fn base64url_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'-' => Some(62),
            b'_' => Some(63),
            _ => None,
        }
    }
    let s = s.trim_end_matches('=');
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut buf: u32 = 0;
    let mut bits = 0;
    for &c in s.as_bytes() {
        buf = (buf << 6) | val(c)?;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((buf >> bits) as u8);
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn fake_jwt(claims: Value) -> String {
        fn enc(v: &Value) -> String {
            // Std base64url without padding via the reverse of our decoder.
            const TBL: &[u8; 64] =
                b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";
            let bytes = serde_json::to_vec(v).unwrap();
            let mut out = String::new();
            for chunk in bytes.chunks(3) {
                let b = [
                    chunk[0],
                    *chunk.get(1).unwrap_or(&0),
                    *chunk.get(2).unwrap_or(&0),
                ];
                let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
                out.push(TBL[(n >> 18) as usize & 63] as char);
                out.push(TBL[(n >> 12) as usize & 63] as char);
                if chunk.len() > 1 {
                    out.push(TBL[(n >> 6) as usize & 63] as char);
                }
                if chunk.len() > 2 {
                    out.push(TBL[n as usize & 63] as char);
                }
            }
            out
        }
        format!("{}.{}.sig", enc(&json!({"alg": "RS256"})), enc(&claims))
    }

    /// Fixture mirrors the real ~/.codex/auth.json layout (captured shape:
    /// auth_mode / OPENAI_API_KEY / tokens{id_token,access_token,
    /// refresh_token,account_id} / last_refresh).
    #[test]
    fn chatgpt_mode_snapshot_carries_email_and_plan() {
        let jwt = fake_jwt(json!({
            "email": "matt@example.com",
            "email_verified": true,
            "https://api.openai.com/auth": {
                "chatgpt_plan_type": "pro",
                "chatgpt_account_id": "acct_1"
            }
        }));
        let raw = json!({
            "auth_mode": "chatgpt",
            "OPENAI_API_KEY": null,
            "tokens": {
                "id_token": jwt,
                "access_token": "at",
                "refresh_token": "rt",
                "account_id": "acct_1"
            },
            "last_refresh": "2026-08-05T12:00:00Z"
        });
        let s = auth_status_from_json(&raw.to_string()).unwrap();
        assert!(s.logged_in);
        assert_eq!(s.auth_mode.as_deref(), Some("chatgpt"));
        assert_eq!(s.email.as_deref(), Some("matt@example.com"));
        assert_eq!(s.plan_type.as_deref(), Some("pro"));
        assert_eq!(s.account_id.as_deref(), Some("acct_1"));
    }

    #[test]
    fn api_key_mode_has_no_identity() {
        let raw = json!({
            "auth_mode": "apikey",
            "OPENAI_API_KEY": "sk-test",
            "tokens": null,
            "last_refresh": null
        });
        let s = auth_status_from_json(&raw.to_string()).unwrap();
        assert!(s.logged_in);
        assert_eq!(s.auth_mode.as_deref(), Some("apikey"));
        assert_eq!(s.email, None);
        assert_eq!(s.plan_type, None);
    }

    #[test]
    fn garbage_jwt_degrades_to_no_label_not_error() {
        let raw = json!({
            "auth_mode": "chatgpt",
            "tokens": {"id_token": "not-a-jwt", "account_id": "a"},
        });
        let s = auth_status_from_json(&raw.to_string()).unwrap();
        assert!(s.logged_in);
        assert_eq!(s.email, None);
        assert_eq!(s.account_id.as_deref(), Some("a"));
    }

    #[test]
    fn base64url_roundtrips_jwt_segments() {
        assert_eq!(base64url_decode("aGVsbG8").unwrap(), b"hello");
        assert_eq!(base64url_decode("aGVsbG8=").unwrap(), b"hello");
        assert!(base64url_decode("!!bad!!").is_none());
    }
}
