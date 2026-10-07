//! The string constants `bridge.gd` exposes, as Rust enums.
//!
//! The JS side speaks snake_case strings, so every enum carries both a
//! `const fn as_str` and a `FromStr` that round-trips.

use std::fmt;
use std::str::FromStr;

/// A string the JS side sent that no variant covers. Kept as text so a newer
/// platform never loses data on the way through.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownWireValue(pub String);

impl fmt::Display for UnknownWireValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown Playgama value {:?}", self.0)
    }
}

impl std::error::Error for UnknownWireValue {}

macro_rules! wire_enum {
    ($(#[$meta:meta])* $name:ident { $($variant:ident => $text:literal),+ $(,)? }) => {
        $(#[$meta])*
        #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
        pub enum $name {
            $($variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $text),+
                }
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(self.as_str())
            }
        }

        impl FromStr for $name {
            type Err = UnknownWireValue;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                match s {
                    $($text => Ok(Self::$variant),)+
                    other => Err(UnknownWireValue(other.to_string())),
                }
            }
        }
    };
}

wire_enum! {
    /// `Bridge.DeviceType`.
    DeviceType {
        Desktop => "desktop",
        Mobile => "mobile",
        Tablet => "tablet",
        Tv => "tv",
    }
}

wire_enum! {
    /// `Bridge.PlatformMessage`, the values `send_message` accepts.
    PlatformMessage {
        GameReady => "game_ready",
        InGameLoadingStarted => "in_game_loading_started",
        InGameLoadingStopped => "in_game_loading_stopped",
        GameplayStarted => "gameplay_started",
        GameplayStopped => "gameplay_stopped",
        PlayerGotAchievement => "player_got_achievement",
        LevelStarted => "level_started",
        LevelCompleted => "level_completed",
        LevelFailed => "level_failed",
        LevelPaused => "level_paused",
        LevelResumed => "level_resumed",
    }
}

wire_enum! {
    /// `Bridge.LeaderboardType`.
    LeaderboardType {
        NotAvailable => "not_available",
        InGame => "in_game",
        Native => "native",
        NativePopup => "native_popup",
    }
}

wire_enum! {
    /// `Bridge.BannerPosition`.
    BannerPosition {
        Top => "top",
        Bottom => "bottom",
    }
}

wire_enum! {
    /// `Bridge.BannerState`.
    BannerState {
        Loading => "loading",
        Shown => "shown",
        Hidden => "hidden",
        Failed => "failed",
    }
}

wire_enum! {
    /// `Bridge.InterstitialState`.
    InterstitialState {
        Loading => "loading",
        Opened => "opened",
        Closed => "closed",
        Failed => "failed",
    }
}

wire_enum! {
    /// `Bridge.RewardedState`.
    RewardedState {
        Loading => "loading",
        Opened => "opened",
        Rewarded => "rewarded",
        Closed => "closed",
        Failed => "failed",
    }
}

wire_enum! {
    /// `Bridge.LaunchSource`.
    LaunchSource {
        Notification => "notification",
        Post => "post",
    }
}

wire_enum! {
    /// `Bridge.PostRewardType`.
    PostRewardType {
        Visit => "visit",
        Author => "author",
    }
}

wire_enum! {
    /// `Bridge.PlatformId`, every platform `platform.id()` can report.
    PlatformId {
        Vk => "vk",
        Ok => "ok",
        Yandex => "yandex",
        CrazyGames => "crazy_games",
        GameDistribution => "game_distribution",
        Playgama => "playgama",
        Standalone => "standalone",
        PlaygamaSandbox => "playgama_sandbox",
        Telegram => "telegram",
        Y8 => "y8",
        Lagged => "lagged",
        Facebook => "facebook",
        Poki => "poki",
        Mock => "mock",
        QaTool => "qa_tool",
        Msn => "msn",
        MicrosoftStore => "microsoft_store",
        Huawei => "huawei",
        GamePush => "gamepush",
        Discord => "discord",
        JioGames => "jio_games",
        YouTube => "youtube",
        Portal => "portal",
        Reddit => "reddit",
        Xiaomi => "xiaomi",
        TikTok => "tiktok",
        Dlightek => "dlightek",
        GameSnacks => "gamesnacks",
        Samsung => "samsung",
    }
}

wire_enum! {
    /// `Bridge.DeviceOs`.
    DeviceOs {
        Windows => "windows",
        MacOs => "macos",
        Linux => "linux",
        Android => "android",
        Ios => "ios",
        Other => "other",
    }
}

wire_enum! {
    /// `Bridge.DeviceOrientation`.
    DeviceOrientation {
        Portrait => "portrait",
        Landscape => "landscape",
    }
}

wire_enum! {
    /// The code on the `BridgeError` a rejected call carries.
    ErrorCode {
        SdkNotInitialized => "SDK_NOT_INITIALIZED",
        InitializationFailed => "INITIALIZATION_FAILED",
        ConfigLoadFailed => "CONFIG_LOAD_FAILED",
        ConfigParseFailed => "CONFIG_PARSE_FAILED",
        StorageNotSupported => "STORAGE_NOT_SUPPORTED",
        StorageNotAvailable => "STORAGE_NOT_AVAILABLE",
        StorageQuotaExceeded => "STORAGE_QUOTA_EXCEEDED",
        GameParamsNotFound => "GAME_PARAMS_NOT_FOUND",
        InviteFriendsMessageLengthError => "INVITE_FRIENDS_MESSAGE_LENGTH_ERROR",
        NotificationsNotSupported => "NOTIFICATIONS_NOT_SUPPORTED",
        NotificationInvalidParameters => "NOTIFICATION_INVALID_PARAMETERS",
    }
}

impl ErrorCode {
    /// The exact text the SDK pairs with the code, so a player reporting a bug
    /// quotes what the platform logged.
    pub const fn message(self) -> &'static str {
        match self {
            Self::SdkNotInitialized => "Before using the SDK you must initialize it",
            Self::InitializationFailed => "SDK initialization failed",
            Self::ConfigLoadFailed => "Failed to load the bridge config file",
            Self::ConfigParseFailed => "Failed to parse the bridge config file",
            Self::StorageNotSupported => "Storage not supported",
            Self::StorageNotAvailable => "Storage not available",
            Self::StorageQuotaExceeded => "Storage quota exceeded",
            Self::GameParamsNotFound => "Game params are not found",
            Self::InviteFriendsMessageLengthError => "Message is too long",
            Self::NotificationsNotSupported => "Notifications not supported",
            Self::NotificationInvalidParameters => "Invalid notification parameters",
        }
    }
}

/// The viewport size `screen_size_changed` reports, in CSS pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ScreenSize {
    pub width: f64,
    pub height: f64,
}

/// The safe-area insets `device.safeArea` reports, in CSS pixels: the margin
/// not covered by notches, rounded corners or platform UI.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SafeArea {
    pub top: f64,
    pub right: f64,
    pub bottom: f64,
    pub left: f64,
}

/// Why a promise call did not deliver its value.
///
/// The SDK rejects with a `BridgeError` carrying an [`ErrorCode`] when it has
/// one to give, and with a bare `Promise.reject()` when a feature is simply
/// unsupported. The two are told apart by [`BridgeError::code`] being `None`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BridgeError {
    code: Option<ErrorCode>,
    raw_code: Option<String>,
    message: Option<String>,
    detail: Option<String>,
}

impl BridgeError {
    /// A rejection with nothing behind it, which is what an unsupported
    /// feature looks like.
    pub fn rejected() -> Self {
        Self::default()
    }

    pub fn new(code: Option<ErrorCode>, message: Option<String>) -> Self {
        Self {
            code,
            message,
            ..Self::default()
        }
    }

    /// The full error, keeping the code text the SDK sent and the
    /// `originalError` it was thrown with. `code` is `None` when the text is
    /// not one this crate knows, and `raw_code` then holds it.
    pub fn detailed(
        code: Option<ErrorCode>,
        raw_code: Option<String>,
        message: Option<String>,
        detail: Option<String>,
    ) -> Self {
        Self {
            code,
            raw_code,
            message,
            detail,
        }
    }

    /// The code the platform reported, or `None` when it rejected without one
    /// or named a code this crate does not know.
    pub fn code(&self) -> Option<ErrorCode> {
        self.code
    }

    /// The code text exactly as the platform sent it, known or not. `None`
    /// only when the platform rejected without naming one.
    pub fn raw_code(&self) -> Option<&str> {
        self.raw_code.as_deref()
    }

    /// What the platform said, falling back to the code's own text. For
    /// `NOTIFICATION_INVALID_PARAMETERS` this is the generic text; the
    /// per-field reason is in [`BridgeError::detail`].
    pub fn message(&self) -> &str {
        match (&self.message, self.code) {
            (Some(message), _) => message,
            (None, Some(code)) => code.message(),
            (None, None) => "the platform refused the call",
        }
    }

    /// The SDK's `originalError`: for notifications, which field was wrong.
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

impl fmt::Display for BridgeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.code {
            Some(code) => write!(f, "{}: {}", code.as_str(), self.message()),
            None => f.write_str(self.message()),
        }
    }
}

impl std::error::Error for BridgeError {}

/// What every promise call in this crate hands its callback.
pub type BridgeResult<T> = Result<T, BridgeError>;

/// One key's worth of cloud storage.
///
/// The JS SDK stores scalars as text, so the wire form of every variant is a
/// string; `serialize` and `deserialize` mirror `utils.gd`'s
/// `serialize_value`/`deserialize_value`.
#[derive(Clone, Debug, PartialEq)]
pub enum StorageValue {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(String),
    Json(serde_json::Value),
}

impl StorageValue {
    /// The text the SDK keeps for this value, or `None` when the write
    /// **deletes** the key. That is the SDK's own rule: a value is deleted when
    /// it is null or the empty string, and a string is stored as it is while
    /// anything else becomes JSON text.
    pub fn stored_text(&self) -> Option<String> {
        match self {
            Self::Null => None,
            Self::Str(text) if text.is_empty() => None,
            Self::Str(text) => Some(text.clone()),
            // `to_js` hands these three to the SDK as `null`, `""` and a string,
            // and the SDK reads all three the way `stored_text` has to.
            Self::Json(serde_json::Value::Null) => None,
            Self::Json(serde_json::Value::String(text)) if text.is_empty() => None,
            Self::Json(serde_json::Value::String(text)) => Some(text.clone()),
            other => Some(number_aware_text(&other.to_json())),
        }
    }

    /// `utils.serialize_value` followed by the storage module's stringify rule:
    /// anything that is not already a string becomes JSON text.
    pub fn serialize(&self) -> String {
        number_aware_text(&self.to_json())
    }

    /// `utils.deserialize_value`. `try_parse_json` is the flag `storage.get`
    /// takes: without it the text comes back exactly as it was stored.
    pub fn deserialize(text: &str, try_parse_json: bool) -> Self {
        if !try_parse_json {
            return Self::Str(text.to_string());
        }
        // An empty value is the SDK's "no value" whatever it holds.
        if text.is_empty() {
            return Self::Null;
        }
        // The SDK reads stored text with `JSON.parse` and falls back to the text
        // itself when that throws, which is exactly what `serde_json` does. So
        // `"1.0"` and `"1e3"` come back as the numbers the platform would return,
        // and `"007"`, `"NaN"` and `"1.10"` stay the text that was stored.
        match serde_json::from_str::<serde_json::Value>(text) {
            Ok(parsed) => Self::from(parsed),
            Err(_) => Self::Str(text.to_string()),
        }
    }

    /// The value as JSON, for handing to the JS side.
    pub fn to_json(&self) -> serde_json::Value {
        match self {
            Self::Null => serde_json::Value::Null,
            Self::Bool(v) => serde_json::Value::Bool(*v),
            Self::Int(v) => serde_json::Value::from(*v),
            Self::Float(v) => serde_json::Number::from_f64(*v)
                .map_or(serde_json::Value::Null, serde_json::Value::Number),
            Self::Str(v) => serde_json::Value::String(v.clone()),
            Self::Json(v) => v.clone(),
        }
    }
}

impl From<serde_json::Value> for StorageValue {
    fn from(value: serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => Self::Null,
            serde_json::Value::Bool(v) => Self::Bool(v),
            serde_json::Value::Number(n) => match n.as_i64() {
                Some(v) => Self::Int(v),
                None => Self::Float(n.as_f64().unwrap_or_default()),
            },
            serde_json::Value::String(v) => Self::Str(v),
            other => Self::Json(other),
        }
    }
}

/// The JSON text for a whole value, with a float written the way
/// `JSON.stringify` writes it: `1.0` becomes `"1"` and `1e21` becomes `"1e+21"`.
/// A value written here and read back is the value that was written.
fn number_aware_text(value: &serde_json::Value) -> String {
    match value {
        // A string is stored as it is, so it must not come back quoted.
        serde_json::Value::String(text) => text.clone(),
        // An integer goes through its own text, not a double's: a `f64` cannot
        // hold every `i64`, so routing this through one would round
        // `i64::MAX` to 9223372036854776000.
        serde_json::Value::Number(number) => match number.as_i64() {
            Some(int) => int.to_string(),
            None => match number.as_f64() {
                Some(float) => float_text(float),
                None => number.to_string(),
            },
        },
        other => other.to_string(),
    }
}

/// `JSON.stringify` for a double. JS switches to exponent form outside
/// `1e-6..1e21` and Rust does not, so the extremes go through it here.
fn float_text(value: f64) -> String {
    if !value.is_finite() {
        // `JSON.stringify(NaN)` and `JSON.stringify(Infinity)` are both "null",
        // which the SDK then reads as a delete.
        return "null".to_string();
    }
    let magnitude = value.abs();
    if magnitude != 0.0 && !(1e-6..1e21).contains(&magnitude) {
        // Rust writes 1e21 as "1000000000000000000000" and 1e-7 as
        // "0.0000001"; JS writes both in exponent form. Both parse back to the
        // same double, so the text only has to be one `JSON.parse` accepts.
        let exponent = format!("{value:e}");
        return exponent.replace('e', "e+").replace("e+-", "e-");
    }
    value.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enum_texts_match_the_js_bridge() {
        assert_eq!(PlatformMessage::GameReady.as_str(), "game_ready");
        assert_eq!(
            "native_popup".parse::<LeaderboardType>().unwrap(),
            LeaderboardType::NativePopup
        );
        assert!("nope".parse::<DeviceType>().is_err());
    }

    #[test]
    fn stored_text_round_trips_the_way_the_js_sdk_writes_it() {
        assert_eq!(StorageValue::Int(5).serialize(), "5");
        assert_eq!(StorageValue::Str("hi".into()).serialize(), "hi");
        assert_eq!(
            StorageValue::Json(serde_json::json!({"a": 1})).serialize(),
            r#"{"a":1}"#
        );
        assert_eq!(
            StorageValue::deserialize("true", true),
            StorageValue::Bool(true)
        );
        assert_eq!(StorageValue::deserialize("5", true), StorageValue::Int(5));
        assert_eq!(
            StorageValue::deserialize("007", true),
            StorageValue::Str("007".into())
        );
        assert_eq!(
            StorageValue::deserialize("[1,2]", true),
            StorageValue::Json(serde_json::json!([1, 2]))
        );
        assert_eq!(
            StorageValue::deserialize(r#"{"a":1}"#, false),
            StorageValue::Str(r#"{"a":1}"#.into())
        );
        assert_eq!(
            StorageValue::deserialize(r#"{"a":1}"#, true),
            StorageValue::Json(serde_json::json!({"a": 1}))
        );
    }

    /// `JSON.stringify` writes a whole double without its decimal point, so
    /// `1.0` is stored as `"1"` and comes back an int. That is the platform, not
    /// this crate. Text no number can round-trip stays text.
    #[test]
    fn floats_follow_the_json_text_and_junk_stays_text() {
        assert_eq!(StorageValue::Float(1.0).serialize(), "1");
        assert_eq!(
            StorageValue::deserialize("1.0", true),
            StorageValue::Float(1.0)
        );
        assert_eq!(
            StorageValue::deserialize("1.5", true),
            StorageValue::Float(1.5)
        );
        assert_eq!(
            StorageValue::deserialize("1e3", true),
            StorageValue::Float(1000.0),
            "JSON.parse accepts an exponent, so the platform returns a number"
        );
        assert_eq!(StorageValue::Float(-0.0).serialize(), "-0");
        assert_eq!(
            StorageValue::deserialize("-0", true),
            StorageValue::Float(-0.0)
        );
        assert_eq!(
            StorageValue::deserialize("NaN", true),
            StorageValue::Str("NaN".into()),
            "JSON.parse throws on NaN, so the SDK hands the text back"
        );
        assert_eq!(
            StorageValue::deserialize("Infinity", true),
            StorageValue::Str("Infinity".into())
        );
        assert_eq!(
            StorageValue::deserialize("", true),
            StorageValue::Null,
            "an empty value is the SDK's no value"
        );
    }

    /// `1e21` and `1e-7` are the two magnitudes where Rust's shortest float
    /// text and `JSON.stringify` disagree; both forms have to read back.
    #[test]
    fn extreme_floats_round_trip_through_either_text() {
        for value in [1e21_f64, 1e-7, f64::MAX, f64::MIN_POSITIVE, 1e-320] {
            let text = StorageValue::Float(value).serialize();
            assert!(
                serde_json::from_str::<serde_json::Value>(&text).is_ok(),
                "{text} is not JSON"
            );
            assert_eq!(
                StorageValue::deserialize(&text, true),
                StorageValue::Float(value),
                "{text} did not come back"
            );
        }
    }

    /// Every `i64` has to survive a write and a read. A 19- or 20-digit one is the
    /// case a length-based guard gets wrong, since both fit in an `i64`.
    #[test]
    fn every_i64_round_trips() {
        for value in [
            0,
            -1,
            42,
            i64::MAX,
            i64::MIN,
            i64::MAX - 1,
            1_000_000_000_000_000_000,
            -1_000_000_000_000_000_000,
        ] {
            let text = StorageValue::Int(value).serialize();
            assert_eq!(
                StorageValue::deserialize(&text, true),
                StorageValue::Int(value),
                "{value} did not come back from {text}"
            );
        }
    }

    /// The SDK deletes a key when the value is null or the empty string, so
    /// both of those have to say so rather than store their text.
    #[test]
    fn null_and_empty_delete_the_key() {
        assert_eq!(StorageValue::Null.stored_text(), None);
        assert_eq!(StorageValue::Str(String::new()).stored_text(), None);
        assert_eq!(
            StorageValue::Json(serde_json::Value::Null).stored_text(),
            None
        );
        assert_eq!(
            StorageValue::Str("hi".into()).stored_text(),
            Some("hi".into()),
            "a string is stored as it is, not as JSON text"
        );
        assert_eq!(
            StorageValue::Json(serde_json::json!("hi")).stored_text(),
            Some("hi".into()),
            "to_js gives the SDK a string, and it stores a string as it is"
        );
        assert_eq!(
            StorageValue::Json(serde_json::json!({"a": 1})).stored_text(),
            Some(r#"{"a":1}"#.into())
        );
    }
}
