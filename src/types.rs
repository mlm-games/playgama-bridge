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
    /// `utils.serialize_value` followed by the storage module's stringify rule:
    /// anything that is not already a string becomes JSON text.
    pub fn serialize(&self) -> String {
        match self {
            Self::Null => "null".to_string(),
            Self::Bool(v) => v.to_string(),
            Self::Int(v) => v.to_string(),
            Self::Float(v) => float_text(*v),
            Self::Str(v) => v.clone(),
            Self::Json(v) => v.to_string(),
        }
    }

    /// `utils.deserialize_value`. `try_parse_json` is the flag `storage.get`
    /// takes: without it the text comes back exactly as it was stored.
    pub fn deserialize(text: &str, try_parse_json: bool) -> Self {
        if !try_parse_json {
            return Self::Str(text.to_string());
        }
        match text {
            "true" => return Self::Bool(true),
            "false" => return Self::Bool(false),
            _ => {}
        }
        // Structured data goes through the parser. The prefix check also keeps
        // the parser from logging an error for every plain value.
        if text.starts_with('{') || text.starts_with('[') {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(text)
                && (parsed.is_object() || parsed.is_array())
            {
                return Self::Json(parsed);
            }
            return Self::Str(text.to_string());
        }
        // Numbers are restored only when the text survives the conversion
        // unchanged, so "007" and "1.10" keep the form they were stored in.
        if text.len() < 19
            && let Ok(v) = text.parse::<i64>()
            && v.to_string() == text
        {
            return Self::Int(v);
        }
        if let Ok(v) = text.parse::<f64>()
            && v.is_finite()
            && float_text(v) == text
        {
            return Self::Float(v);
        }
        Self::Str(text.to_string())
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

/// `JSON.stringify` keeps a whole float's decimal point, so `1.0` stores as
/// `"1.0"` and comes back a float rather than an int.
fn float_text(value: f64) -> String {
    let text = value.to_string();
    if text.contains('.') || text.contains(['e', 'E']) {
        text
    } else {
        format!("{text}.0")
    }
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

    /// A whole float keeps its decimal point, so it does not come back as an
    /// int, and text no float can round-trip stays text.
    #[test]
    fn floats_keep_their_type_and_junk_stays_text() {
        assert_eq!(StorageValue::Float(1.0).serialize(), "1.0");
        assert_eq!(
            StorageValue::deserialize("1.0", true),
            StorageValue::Float(1.0)
        );
        assert_eq!(
            StorageValue::deserialize("-0", true),
            StorageValue::Str("-0".into())
        );
        assert_eq!(
            StorageValue::deserialize("NaN", true),
            StorageValue::Str("NaN".into())
        );
        assert_eq!(
            StorageValue::deserialize("inf", true),
            StorageValue::Str("inf".into())
        );
    }
}
