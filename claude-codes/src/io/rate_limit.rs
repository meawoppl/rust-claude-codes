use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;

/// Current rate limit disposition.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitStatus {
    /// Request is within limits.
    Allowed,
    /// Request is within limits but approaching the cap.
    AllowedWarning,
    /// Request was rejected due to rate limiting.
    Rejected,
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl RateLimitStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Allowed => "allowed",
            Self::AllowedWarning => "allowed_warning",
            Self::Rejected => "rejected",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for RateLimitStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RateLimitStatus {
    fn from(s: &str) -> Self {
        match s {
            "allowed" => Self::Allowed,
            "allowed_warning" => Self::AllowedWarning,
            "rejected" => Self::Rejected,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for RateLimitStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RateLimitStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// The time window a rate limit applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitWindow {
    /// Five-hour rolling window.
    FiveHour,
    /// Hourly rolling window.
    Hourly,
    /// Seven-day rolling window.
    SevenDay,
    /// A window type not yet known to this version of the crate.
    Unknown(String),
}

impl RateLimitWindow {
    pub fn as_str(&self) -> &str {
        match self {
            Self::FiveHour => "five_hour",
            Self::Hourly => "hourly",
            Self::SevenDay => "seven_day",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for RateLimitWindow {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RateLimitWindow {
    fn from(s: &str) -> Self {
        match s {
            "five_hour" => Self::FiveHour,
            "hourly" => Self::Hourly,
            "seven_day" => Self::SevenDay,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for RateLimitWindow {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RateLimitWindow {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Whether overage billing was accepted or rejected.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OverageStatus {
    /// Overage was accepted.
    Allowed,
    /// Overage was rejected.
    Rejected,
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl OverageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Allowed => "allowed",
            Self::Rejected => "rejected",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for OverageStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for OverageStatus {
    fn from(s: &str) -> Self {
        match s {
            "allowed" => Self::Allowed,
            "rejected" => Self::Rejected,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for OverageStatus {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OverageStatus {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

/// Why overage billing is disabled.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum OverageDisabledReason {
    /// Overage is disabled at the organization level.
    OrgLevelDisabled,
    /// The account is out of credits.
    OutOfCredits,
    /// A reason not yet known to this version of the crate.
    Unknown(String),
}

impl OverageDisabledReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::OrgLevelDisabled => "org_level_disabled",
            Self::OutOfCredits => "out_of_credits",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for OverageDisabledReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for OverageDisabledReason {
    fn from(s: &str) -> Self {
        match s {
            "org_level_disabled" => Self::OrgLevelDisabled,
            "out_of_credits" => Self::OutOfCredits,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for OverageDisabledReason {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for OverageDisabledReason {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

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
    /// Current rate limit status
    pub status: RateLimitStatus,
    /// Unix timestamp when the rate limit resets
    #[serde(rename = "resetsAt")]
    pub resets_at: u64,
    /// Type of rate limit window
    #[serde(rename = "rateLimitType")]
    pub rate_limit_type: RateLimitWindow,
    /// Utilization of the rate limit (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utilization: Option<f64>,
    /// Overage status (e.g., rejected, allowed)
    #[serde(skip_serializing_if = "Option::is_none", rename = "overageStatus")]
    pub overage_status: Option<OverageStatus>,
    /// Reason overage is disabled, if applicable
    #[serde(rename = "overageDisabledReason")]
    pub overage_disabled_reason: Option<OverageDisabledReason>,
    /// Whether overage billing is active
    #[serde(rename = "isUsingOverage")]
    pub is_using_overage: bool,
}

#[cfg(test)]
mod tests {
    use super::{OverageDisabledReason, OverageStatus, RateLimitStatus, RateLimitWindow};
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
        assert_eq!(evt.rate_limit_info.status, RateLimitStatus::Allowed);
        assert_eq!(evt.rate_limit_info.resets_at, 1771390800);
        assert_eq!(
            evt.rate_limit_info.rate_limit_type,
            RateLimitWindow::FiveHour
        );
        assert_eq!(evt.rate_limit_info.utilization, None);
        assert_eq!(
            evt.rate_limit_info.overage_status,
            Some(OverageStatus::Rejected)
        );
        assert_eq!(
            evt.rate_limit_info.overage_disabled_reason,
            Some(OverageDisabledReason::OrgLevelDisabled)
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
        assert_eq!(evt.rate_limit_info.status, RateLimitStatus::AllowedWarning);
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
        assert_eq!(evt.rate_limit_info.status, RateLimitStatus::Rejected);
        assert_eq!(
            evt.rate_limit_info.rate_limit_type,
            RateLimitWindow::SevenDay
        );
        assert_eq!(
            evt.rate_limit_info.overage_status,
            Some(OverageStatus::Rejected)
        );
        assert_eq!(
            evt.rate_limit_info.overage_disabled_reason,
            Some(OverageDisabledReason::OutOfCredits)
        );
    }
}
