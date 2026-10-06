//! The fourteen module interfaces `bridge.gd` exposes.
//!
//! One trait per module, implemented once against the browser SDK
//! (`crate::js`) and once against the editor stand-ins (`crate::mock`), so
//! game code is written once and runs on the web and on the desktop.

use crate::signal::Signal;
use crate::types::{
    BannerPosition, BannerState, DeviceType, InterstitialState, LaunchSource, LeaderboardType,
    PlatformMessage, RewardedState, StorageValue,
};

/// A JS object copied across the FFI, the shape `convert_to_gd_object` returns.
pub type ObjectMap = serde_json::Map<String, serde_json::Value>;

pub trait PlatformApi {
    /// Platform id (`"mock"` off the web).
    fn id(&self) -> Option<String>;
    /// The post payload the game was opened from, when there is one.
    fn payload(&self) -> Option<String>;
    fn language(&self) -> Option<String>;
    /// Top-level domain, absent on some platforms.
    fn tld(&self) -> Option<String>;
    fn launch_source(&self) -> Option<LaunchSource>;
    /// Everything the launch carries.
    fn data(&self) -> ObjectMap;
    fn is_audio_enabled(&self) -> bool;
    fn is_external_calls_supported(&self) -> bool;
    fn is_external_links_allowed(&self) -> bool;

    fn send_message(&self, message: PlatformMessage);
    fn send_message_with(&self, message: PlatformMessage, options: &serde_json::Value);
    fn send_custom_message(&self, id: &str);
    fn send_custom_message_with(&self, id: &str, options: &serde_json::Value);

    /// Unix milliseconds, or `0` when the platform refuses to answer.
    fn get_server_time(&self, callback: Box<dyn FnOnce(i64)>);

    /// `false` while the platform has audio off.
    fn audio_state_changed(&self) -> &Signal<bool>;
    fn pause_state_changed(&self) -> &Signal<bool>;
}

pub trait DeviceApi {
    fn device_type(&self) -> DeviceType;
}

pub trait PlayerApi {
    fn is_authorization_supported(&self) -> bool;
    fn is_authorized(&self) -> bool;
    fn is_guest(&self) -> bool;
    fn id(&self) -> Option<String>;
    fn name(&self) -> Option<String>;
    fn photos(&self) -> Vec<String>;
    fn extra(&self) -> ObjectMap;

    fn authorize(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>);
}

pub trait StorageApi {
    /// One entry per key, in the order asked for. The callback's flag is
    /// `false` when the platform rejected the whole read.
    fn get(
        &self,
        keys: &[&str],
        try_parse_json: bool,
        callback: Box<dyn FnOnce(bool, Vec<StorageValue>)>,
    );
    fn set(&self, pairs: &[(&str, StorageValue)], callback: Box<dyn FnOnce(bool)>);
    fn delete(&self, keys: &[&str], callback: Box<dyn FnOnce(bool)>);
}

pub trait AdvertisementApi {
    fn minimum_delay_between_interstitial(&self) -> f64;
    fn set_minimum_delay_between_interstitial(&self, seconds: f64);
    fn is_banner_supported(&self) -> bool;
    fn banner_state(&self) -> Option<BannerState>;
    fn is_interstitial_supported(&self) -> bool;
    fn interstitial_state(&self) -> Option<InterstitialState>;
    fn is_rewarded_supported(&self) -> bool;
    fn rewarded_state(&self) -> Option<RewardedState>;
    fn rewarded_placement(&self) -> Option<String>;
    fn is_advanced_banners_supported(&self) -> bool;
    fn advanced_banners_state(&self) -> Option<BannerState>;

    fn show_banner(&self, position: BannerPosition, placement: Option<&str>);
    fn hide_banner(&self);
    fn show_interstitial(&self, placement: Option<&str>);
    fn show_rewarded(&self, placement: Option<&str>);
    fn show_advanced_banners(&self, placement: Option<&str>);
    fn hide_advanced_banners(&self);

    fn check_adblock(&self, callback: Box<dyn FnOnce(bool)>);

    fn banner_state_changed(&self) -> &Signal<Option<BannerState>>;
    fn interstitial_state_changed(&self) -> &Signal<Option<InterstitialState>>;
    fn rewarded_state_changed(&self) -> &Signal<Option<RewardedState>>;
    fn advanced_banners_state_changed(&self) -> &Signal<Option<BannerState>>;
}

pub trait SocialApi {
    fn is_share_supported(&self) -> bool;
    fn is_join_community_supported(&self) -> bool;
    fn is_invite_friends_supported(&self) -> bool;
    fn is_create_post_supported(&self) -> bool;
    fn is_add_to_favorites_supported(&self) -> bool;
    fn is_add_to_home_screen_supported(&self) -> bool;
    fn is_rate_supported(&self) -> bool;
    fn is_post_reward_supported(&self) -> bool;

    /// `options` is either an entry id from `playgama-bridge-config.json` or the
    /// content to share.
    fn share(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>);
    fn join_community(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>);
    fn invite_friends(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>);
    /// `payload` is the game's own string for this one post — a level, a seed,
    /// a challenge — handed back as `Bridge.platform.payload`.
    fn create_post(
        &self,
        options: Option<&serde_json::Value>,
        payload: Option<&str>,
        callback: Box<dyn FnOnce(bool)>,
    );
    fn add_to_favorites(&self, callback: Box<dyn FnOnce(bool)>);
    fn add_to_home_screen(&self, callback: Box<dyn FnOnce(bool)>);
    fn rate(&self, callback: Box<dyn FnOnce(bool)>);
    fn get_post_reward(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
}

pub trait LeaderboardsApi {
    fn leaderboard_type(&self) -> LeaderboardType;
    fn set_score(&self, id: &str, score: f64, callback: Box<dyn FnOnce(bool)>);
    fn get_entries(&self, id: &str, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
    fn show_native_popup(&self, id: &str, callback: Box<dyn FnOnce(bool)>);
}

pub trait PaymentsApi {
    fn is_supported(&self) -> bool;
    fn purchase(
        &self,
        id: &str,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>,
    );
    fn consume_purchase(&self, id: &str, callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>);
    fn get_catalog(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
    fn get_purchases(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
}

pub trait AchievementsApi {
    fn unlock(&self, id: &str, callback: Box<dyn FnOnce(bool)>);
    fn get_achievements(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
}

pub trait RemoteConfigApi {
    fn is_supported(&self) -> bool;
    fn set_context(&self, parameters: &serde_json::Value);
    fn get(&self, callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>);
}

pub trait CrossPromoApi {
    fn is_visible(&self) -> bool;
    fn get_games(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>);
    fn show(&self);
    fn hide(&self);
}

pub trait TasksApi {
    fn get_tasks(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>);
    fn add_progress(&self, metric: &str, amount: i64, callback: Box<dyn FnOnce(bool)>);
    fn claim_reward(&self, task_id: &str, callback: Box<dyn FnOnce(bool)>);
}

pub trait DailyRewardsApi {
    fn get_rewards(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>);
    /// `0` when the platform refuses to answer.
    fn get_current_day(&self, callback: Box<dyn FnOnce(bool, i64)>);
    /// The claimable reward id, or `None` when nothing can be claimed.
    fn get_current_reward(&self, callback: Box<dyn FnOnce(bool, Option<String>)>);
    fn claim_current_reward(&self, callback: Box<dyn FnOnce(bool)>);
}

pub trait NotificationsApi {
    fn is_supported(&self) -> bool;
    fn schedule(&self, notification: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>);
    fn cancel(&self, id: &str, callback: Box<dyn FnOnce(bool)>);
    fn cancel_all(&self, callback: Box<dyn FnOnce(bool)>);
}
