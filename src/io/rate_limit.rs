use serde::{Deserialize, Serialize};

/// Rate limit event from Claude CLI.
///
/// Sent periodically to inform consumers about current rate limit status,
/// including overage eligibility and reset timing.
///
/// # Example JSON
///
/// ```json
/// {
///   "type": "rate_limit_event",
///   "rate_limit_info": {
///     "status": "allowed",
///     "resetsAt": 1771390800,
///     "rateLimitType": "five_hour",
///     "overageStatus": "rejected",
///     "overageDisabledReason": "org_level_disabled",
///     "isUsingOverage": false
///   },
///   "uuid": "76258cfb-0dc8-4d4b-8682-77082b59c03f",
///   "session_id": "1ae0af5b-89fa-4075-8156-d5d3702f6505"
/// }
/// ```
///
/// # Example
///
/// ```
/// use claude_codes::ClaudeOutput;
///
/// let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1771390800,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false},"uuid":"abc","session_id":"def"}"#;
/// let output: ClaudeOutput = serde_json::from_str(json).unwrap();
///
/// if let Some(evt) = output.as_rate_limit_event() {
///     println!("Rate limit status: {}", evt.rate_limit_info.status);
///     println!("Resets at: {}", evt.rate_limit_info.resets_at);
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEvent {
    /// Rate limit status details
    pub rate_limit_info: RateLimitInfo,
    /// Session identifier
    pub session_id: String,
    /// Unique identifier for this message
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uuid: Option<String>,
}

/// Rate limit status information.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitInfo {
    /// Current rate limit status (e.g., "allowed")
    pub status: String,
    /// Unix timestamp when the rate limit resets
    #[serde(rename = "resetsAt")]
    pub resets_at: u64,
    /// Type of rate limit (e.g., "five_hour")
    #[serde(rename = "rateLimitType")]
    pub rate_limit_type: String,
    /// Utilization of the rate limit (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utilization: Option<f64>,
    /// Overage status (e.g., "rejected", "allowed")
    #[serde(skip_serializing_if = "Option::is_none", rename = "overageStatus")]
    pub overage_status: Option<String>,
    /// Reason overage is disabled, if applicable
    #[serde(rename = "overageDisabledReason")]
    pub overage_disabled_reason: Option<String>,
    /// Whether overage billing is active
    #[serde(rename = "isUsingOverage")]
    pub is_using_overage: bool,
}

#[cfg(test)]
mod tests {
    use crate::io::ClaudeOutput;

    #[test]
    fn test_deserialize_rate_limit_event() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":1771390800,"rateLimitType":"five_hour","overageStatus":"rejected","overageDisabledReason":"org_level_disabled","isUsingOverage":false},"uuid":"76258cfb-0dc8-4d4b-8682-77082b59c03f","session_id":"1ae0af5b-89fa-4075-8156-d5d3702f6505"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        assert!(output.is_rate_limit_event());
        assert_eq!(output.message_type(), "rate_limit_event");
        assert_eq!(
            output.session_id(),
            Some("1ae0af5b-89fa-4075-8156-d5d3702f6505")
        );

        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.status, "allowed");
        assert_eq!(evt.rate_limit_info.resets_at, 1771390800);
        assert_eq!(evt.rate_limit_info.rate_limit_type, "five_hour");
        assert_eq!(evt.rate_limit_info.utilization, None);
        assert_eq!(
            evt.rate_limit_info.overage_status,
            Some("rejected".to_string())
        );
        assert_eq!(
            evt.rate_limit_info.overage_disabled_reason,
            Some("org_level_disabled".to_string())
        );
        assert!(!evt.rate_limit_info.is_using_overage);
        assert_eq!(
            evt.uuid,
            Some("76258cfb-0dc8-4d4b-8682-77082b59c03f".to_string())
        );
    }

    #[test]
    fn test_deserialize_rate_limit_event_minimal() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","resetsAt":0,"rateLimitType":"hourly","overageStatus":"allowed","isUsingOverage":true},"session_id":"abc"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.overage_disabled_reason, None);
        assert!(evt.rate_limit_info.is_using_overage);
        assert!(evt.uuid.is_none());
    }

    #[test]
    fn test_deserialize_rate_limit_event_allowed_warning() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","resetsAt":1700000000,"rateLimitType":"five_hour","utilization":0.85,"isUsingOverage":false},"uuid":"550e8400-e29b-41d4-a716-446655440000","session_id":"test-session-id"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.status, "allowed_warning");
        assert_eq!(evt.rate_limit_info.utilization, Some(0.85));
        assert_eq!(evt.rate_limit_info.overage_status, None);
        assert_eq!(evt.rate_limit_info.overage_disabled_reason, None);
        assert!(!evt.rate_limit_info.is_using_overage);
    }

    #[test]
    fn test_deserialize_rate_limit_event_rejected() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1700003600,"rateLimitType":"seven_day","isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"out_of_credits"},"uuid":"660e8400-e29b-41d4-a716-446655440001","session_id":"test-session-id"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.status, "rejected");
        assert_eq!(evt.rate_limit_info.rate_limit_type, "seven_day");
        assert_eq!(
            evt.rate_limit_info.overage_status,
            Some("rejected".to_string())
        );
        assert_eq!(
            evt.rate_limit_info.overage_disabled_reason,
            Some("out_of_credits".to_string())
        );
    }
}
