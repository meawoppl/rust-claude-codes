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
    /// Seven-day rolling window covering all models.
    SevenDay,
    /// Seven-day rolling window scoped to Opus usage.
    SevenDayOpus,
    /// Seven-day rolling window scoped to Sonnet usage.
    SevenDaySonnet,
    /// Seven-day rolling window for models included in overage billing.
    SevenDayOverageIncluded,
    /// Overage (extra usage / usage credits) window.
    Overage,
    /// A window type not yet known to this version of the crate.
    Unknown(String),
}

impl RateLimitWindow {
    pub fn as_str(&self) -> &str {
        match self {
            Self::FiveHour => "five_hour",
            Self::SevenDay => "seven_day",
            Self::SevenDayOpus => "seven_day_opus",
            Self::SevenDaySonnet => "seven_day_sonnet",
            Self::SevenDayOverageIncluded => "seven_day_overage_included",
            Self::Overage => "overage",
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
            "seven_day" => Self::SevenDay,
            "seven_day_opus" => Self::SevenDayOpus,
            "seven_day_sonnet" => Self::SevenDaySonnet,
            "seven_day_overage_included" => Self::SevenDayOverageIncluded,
            "overage" => Self::Overage,
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
    /// Overage was accepted but usage is approaching the cap.
    AllowedWarning,
    /// Overage was rejected.
    Rejected,
    /// A status not yet known to this version of the crate.
    Unknown(String),
}

impl OverageStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Allowed => "allowed",
            Self::AllowedWarning => "allowed_warning",
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
            "allowed_warning" => Self::AllowedWarning,
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
    /// Overage has not been provisioned for the account.
    OverageNotProvisioned,
    /// Overage is disabled at the organization level.
    OrgLevelDisabled,
    /// Overage is disabled at the organization level until a later time.
    OrgLevelDisabledUntil,
    /// The account is out of credits.
    OutOfCredits,
    /// Overage is disabled for this seat tier.
    SeatTierLevelDisabled,
    /// Overage is disabled for this member.
    MemberLevelDisabled,
    /// The seat tier has a zero credit limit.
    SeatTierZeroCreditLimit,
    /// The member's group has a zero credit limit.
    GroupZeroCreditLimit,
    /// The member has a zero credit limit.
    MemberZeroCreditLimit,
    /// Overage is disabled at the organization service level.
    OrgServiceLevelDisabled,
    /// No overage limits are configured.
    NoLimitsConfigured,
    /// The server failed to fetch the overage configuration.
    FetchError,
    /// A reason not yet known to this version of the crate (including the
    /// server's own literal `"unknown"` value).
    Unknown(String),
}

impl OverageDisabledReason {
    pub fn as_str(&self) -> &str {
        match self {
            Self::OverageNotProvisioned => "overage_not_provisioned",
            Self::OrgLevelDisabled => "org_level_disabled",
            Self::OrgLevelDisabledUntil => "org_level_disabled_until",
            Self::OutOfCredits => "out_of_credits",
            Self::SeatTierLevelDisabled => "seat_tier_level_disabled",
            Self::MemberLevelDisabled => "member_level_disabled",
            Self::SeatTierZeroCreditLimit => "seat_tier_zero_credit_limit",
            Self::GroupZeroCreditLimit => "group_zero_credit_limit",
            Self::MemberZeroCreditLimit => "member_zero_credit_limit",
            Self::OrgServiceLevelDisabled => "org_service_level_disabled",
            Self::NoLimitsConfigured => "no_limits_configured",
            Self::FetchError => "fetch_error",
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
            "overage_not_provisioned" => Self::OverageNotProvisioned,
            "org_level_disabled" => Self::OrgLevelDisabled,
            "org_level_disabled_until" => Self::OrgLevelDisabledUntil,
            "out_of_credits" => Self::OutOfCredits,
            "seat_tier_level_disabled" => Self::SeatTierLevelDisabled,
            "member_level_disabled" => Self::MemberLevelDisabled,
            "seat_tier_zero_credit_limit" => Self::SeatTierZeroCreditLimit,
            "group_zero_credit_limit" => Self::GroupZeroCreditLimit,
            "member_zero_credit_limit" => Self::MemberZeroCreditLimit,
            "org_service_level_disabled" => Self::OrgServiceLevelDisabled,
            "no_limits_configured" => Self::NoLimitsConfigured,
            "fetch_error" => Self::FetchError,
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
///     if let Some(resets_at) = evt.rate_limit_info.resets_at {
///         println!("Resets at: {}", resets_at);
///     }
/// }
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitEvent {
    /// Rate limit status details
    pub rate_limit_info: RateLimitInfo,
    /// Session identifier
    #[serde(alias = "sessionId")]
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
    #[serde(rename = "resetsAt", skip_serializing_if = "Option::is_none")]
    pub resets_at: Option<u64>,
    /// Type of rate limit window
    #[serde(rename = "rateLimitType", skip_serializing_if = "Option::is_none")]
    pub rate_limit_type: Option<RateLimitWindow>,
    /// Utilization of the rate limit (0.0 to 1.0)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub utilization: Option<f64>,
    /// Overage status (e.g., rejected, allowed)
    #[serde(skip_serializing_if = "Option::is_none", rename = "overageStatus")]
    pub overage_status: Option<OverageStatus>,
    /// Unix timestamp when the overage window resets
    #[serde(rename = "overageResetsAt", skip_serializing_if = "Option::is_none")]
    pub overage_resets_at: Option<u64>,
    /// Reason overage is disabled, if applicable
    #[serde(
        rename = "overageDisabledReason",
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_disabled_reason: Option<OverageDisabledReason>,
    /// Whether overage billing is active for the current request
    #[serde(rename = "isUsingOverage", skip_serializing_if = "Option::is_none")]
    pub is_using_overage: Option<bool>,
    /// Whether overage billing is in use for the account
    #[serde(rename = "overageInUse", skip_serializing_if = "Option::is_none")]
    pub overage_in_use: Option<bool>,
    /// Utilization warning threshold that was crossed (0.0 to 1.0)
    #[serde(rename = "surpassedThreshold", skip_serializing_if = "Option::is_none")]
    pub surpassed_threshold: Option<f64>,
    /// Monthly service spend-cap utilization (Claude-in-Slack surface)
    #[serde(
        rename = "overagePeriodMonthly",
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_period_monthly: Option<OveragePeriodUtilization>,
    /// Per-channel spend-cap utilization (Claude-in-Slack surface)
    #[serde(
        rename = "overagePeriodChannel",
        skip_serializing_if = "Option::is_none"
    )]
    pub overage_period_channel: Option<OveragePeriodUtilization>,
    /// Error code attached when a request was refused outright
    #[serde(rename = "errorCode", skip_serializing_if = "Option::is_none")]
    pub error_code: Option<RateLimitErrorCode>,
    /// Whether the user is able to purchase credits themselves
    #[serde(
        rename = "canUserPurchaseCredits",
        skip_serializing_if = "Option::is_none"
    )]
    pub can_user_purchase_credits: Option<bool>,
    /// Whether the account has a chargeable saved payment method
    #[serde(
        rename = "hasChargeableSavedPaymentMethod",
        skip_serializing_if = "Option::is_none"
    )]
    pub has_chargeable_saved_payment_method: Option<bool>,
    /// Per-window usage for the subscription rate-limit windows, as read from
    /// the `anthropic-ratelimit-unified-*` response headers. Unlike the
    /// top-level `status`/`utilization` fields (which describe the currently
    /// limiting window), every window is tracked on each observation, and
    /// events are emitted when a window's rounded percentage or reset time
    /// moves. Absent until the first response carrying the headers, and
    /// always absent for API-key, Bedrock, and Vertex sessions (CLI 2.1.258+).
    #[serde(rename = "unifiedWindows", skip_serializing_if = "Option::is_none")]
    pub unified_windows: Option<UnifiedWindows>,
}

/// Per-window subscription usage carried on a rate limit event (see
/// [`RateLimitInfo::unified_windows`]). Windows absent from the account
/// state are absent here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedWindows {
    /// The session (5-hour) window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub five_hour: Option<UnifiedWindowUsage>,
    /// The weekly (7-day) window.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seven_day: Option<UnifiedWindowUsage>,
    /// The overage-included weekly window (per-model bucket; present only
    /// for accounts whose responses carry that window).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seven_day_overage_included: Option<UnifiedWindowUsage>,
}

/// Usage of a single subscription rate-limit window.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UnifiedWindowUsage {
    /// Fraction of the window used — usually 0.0 to 1.0, but values above 1
    /// occur when usage legitimately runs past a window's cap.
    pub utilization: f64,
    /// Unix epoch seconds when the window resets.
    #[serde(rename = "resetsAt")]
    pub resets_at: u64,
}

/// Spend-cap utilization for an overage billing period.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OveragePeriodUtilization {
    /// Fraction of the spend cap consumed (0.0 to 1.0)
    pub utilization: f64,
}

/// Error code carried on a rate limit event when a request was refused.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RateLimitErrorCode {
    /// The request requires purchasing usage credits.
    CreditsRequired,
    /// An error code not yet known to this version of the crate.
    Unknown(String),
}

impl RateLimitErrorCode {
    pub fn as_str(&self) -> &str {
        match self {
            Self::CreditsRequired => "credits_required",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

impl fmt::Display for RateLimitErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl From<&str> for RateLimitErrorCode {
    fn from(s: &str) -> Self {
        match s {
            "credits_required" => Self::CreditsRequired,
            other => Self::Unknown(other.to_string()),
        }
    }
}

impl Serialize for RateLimitErrorCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for RateLimitErrorCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        Ok(Self::from(s.as_str()))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OverageDisabledReason, OveragePeriodUtilization, OverageStatus, RateLimitErrorCode,
        RateLimitStatus, RateLimitWindow,
    };
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
        assert_eq!(evt.rate_limit_info.resets_at, Some(1771390800));
        assert_eq!(
            evt.rate_limit_info.rate_limit_type,
            Some(RateLimitWindow::FiveHour)
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
        assert_eq!(evt.rate_limit_info.is_using_overage, Some(false));
        assert_eq!(
            evt.uuid,
            Some("76258cfb-0dc8-4d4b-8682-77082b59c03f".to_string())
        );
    }

    #[test]
    fn test_deserialize_rate_limit_event_minimal() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed"},"session_id":"abc"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.overage_disabled_reason, None);
        assert_eq!(evt.rate_limit_info.is_using_overage, None);
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
        assert_eq!(evt.rate_limit_info.is_using_overage, Some(false));
    }

    #[test]
    fn test_deserialize_rate_limit_event_no_resets_at() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","isUsingOverage":false},"uuid":"4269273d-3b5e-40ae-9765-cb3c12284c44","session_id":"f9626cf7-4d88-4844-9bb1-cab96909fc7b"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.status, RateLimitStatus::Allowed);
        assert_eq!(evt.rate_limit_info.resets_at, None);
        assert_eq!(evt.rate_limit_info.rate_limit_type, None);
        assert_eq!(evt.rate_limit_info.is_using_overage, Some(false));
    }

    #[test]
    fn test_deserialize_rate_limit_event_rejected() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1700003600,"rateLimitType":"seven_day","isUsingOverage":false,"overageStatus":"rejected","overageDisabledReason":"out_of_credits"},"uuid":"660e8400-e29b-41d4-a716-446655440001","session_id":"test-session-id"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        assert_eq!(evt.rate_limit_info.status, RateLimitStatus::Rejected);
        assert_eq!(
            evt.rate_limit_info.rate_limit_type,
            Some(RateLimitWindow::SevenDay)
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

    #[test]
    fn test_deserialize_rate_limit_event_full_2_1_205() {
        let json = r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","resetsAt":1700003600,"rateLimitType":"seven_day_overage_included","utilization":0.97,"overageStatus":"allowed_warning","overageResetsAt":1700007200,"overageDisabledReason":"seat_tier_zero_credit_limit","isUsingOverage":true,"overageInUse":true,"surpassedThreshold":0.8,"overagePeriodMonthly":{"utilization":0.5},"overagePeriodChannel":{"utilization":0.25},"errorCode":"credits_required","canUserPurchaseCredits":true,"hasChargeableSavedPaymentMethod":false},"session_id":"abc"}"#;

        let output: ClaudeOutput = serde_json::from_str(json).unwrap();
        let evt = output.as_rate_limit_event().unwrap();
        let info = &evt.rate_limit_info;
        assert_eq!(
            info.rate_limit_type,
            Some(RateLimitWindow::SevenDayOverageIncluded)
        );
        assert_eq!(info.overage_status, Some(OverageStatus::AllowedWarning));
        assert_eq!(info.overage_resets_at, Some(1700007200));
        assert_eq!(
            info.overage_disabled_reason,
            Some(OverageDisabledReason::SeatTierZeroCreditLimit)
        );
        assert_eq!(info.is_using_overage, Some(true));
        assert_eq!(info.overage_in_use, Some(true));
        assert_eq!(info.surpassed_threshold, Some(0.8));
        assert_eq!(
            info.overage_period_monthly,
            Some(OveragePeriodUtilization { utilization: 0.5 })
        );
        assert_eq!(
            info.overage_period_channel,
            Some(OveragePeriodUtilization { utilization: 0.25 })
        );
        assert_eq!(info.error_code, Some(RateLimitErrorCode::CreditsRequired));
        assert_eq!(info.can_user_purchase_credits, Some(true));
        assert_eq!(info.has_chargeable_saved_payment_method, Some(false));

        let round_trip = serde_json::to_value(output).unwrap();
        let original: serde_json::Value = serde_json::from_str(json).unwrap();
        assert_eq!(round_trip, original);
    }

    #[test]
    fn test_rate_limit_window_round_trip() {
        for s in [
            "five_hour",
            "seven_day",
            "seven_day_opus",
            "seven_day_sonnet",
            "seven_day_overage_included",
            "overage",
            "some_future_window",
        ] {
            assert_eq!(RateLimitWindow::from(s).as_str(), s);
        }
    }

    #[test]
    fn test_overage_disabled_reason_round_trip() {
        for s in [
            "overage_not_provisioned",
            "org_level_disabled",
            "org_level_disabled_until",
            "out_of_credits",
            "seat_tier_level_disabled",
            "member_level_disabled",
            "seat_tier_zero_credit_limit",
            "group_zero_credit_limit",
            "member_zero_credit_limit",
            "org_service_level_disabled",
            "no_limits_configured",
            "fetch_error",
            "unknown",
        ] {
            assert_eq!(OverageDisabledReason::from(s).as_str(), s);
        }
    }
}
