//! The module interfaces `bridge.gd` exposes, plus the root object they hang
//! off.
//!
//! One trait per module, implemented once against the browser SDK
//! (`crate::js`) and once against the editor stand-ins (`crate::mock`), so
//! game code is written once and runs on the web and on the desktop.

use crate::signal::Signal;
use crate::types::{
    BannerPosition, BannerState, BridgeResult, DeviceOrientation, DeviceOs, DeviceType,
    InterstitialState, LaunchSource, LeaderboardType, PlatformId, PlatformMessage, RewardedState,
    SafeArea, ScreenSize, StorageValue,
};

/// A JS object copied across the FFI, the shape `convert_to_gd_object` returns.
pub type ObjectMap = serde_json::Map<String, serde_json::Value>;

/// The root object every module hangs off, `window.bridge` itself.
pub trait RootApi {
    /// The SDK version string, `"2.3.0"` on the current stable.
    fn version(&self) -> Option<String>;
    fn is_initialized(&self) -> bool;
    /// The merged `playgama-bridge-config.json` every module reads its
    /// placements, achievements, product ids and task tables from.
    fn options(&self) -> ObjectMap;
    /// The game version analytics events are tagged with. Set it before
    /// [`crate::load`] returns if you want it on the first batch.
    fn set_game_version(&self, version: &str);
}

pub trait PlatformApi {
    /// Platform id exactly as the platform spells it. The SDK falls back to
    /// `"mock"` for any host it does not recognise, so this reads `"mock"` on
    /// the web too when the game is served from somewhere unexpected.
    fn id(&self) -> Option<String>;
    /// The same id as a constant, or `None` for a platform newer than this
    /// crate. [`PlatformApi::id`] still has the text.
    fn platform_id(&self) -> Option<PlatformId>;
    /// The underlying platform SDK's own version, where it reports one.
    fn sdk(&self) -> Option<String>;
    /// The post payload the game was opened from, when there is one.
    fn payload(&self) -> Option<String>;
    fn language(&self) -> Option<String>;
    /// Top-level domain, absent on some platforms.
    fn tld(&self) -> Option<String>;
    fn launch_source(&self) -> Option<LaunchSource>;
    /// Everything the launch carries, `postId` included.
    fn data(&self) -> ObjectMap;
    fn is_audio_enabled(&self) -> bool;
    /// Read this at start: the signal only reports changes.
    fn is_paused(&self) -> bool;
    fn is_external_calls_supported(&self) -> bool;
    fn is_external_links_allowed(&self) -> bool;

    /// Fire and forget, as `bridge.gd` calls it. The SDK rejects a second
    /// `game_ready`, and nothing here can report that.
    fn send_message(&self, message: PlatformMessage);
    fn send_message_with(&self, message: PlatformMessage, options: &serde_json::Value);
    fn send_custom_message(&self, id: &str);
    fn send_custom_message_with(&self, id: &str, options: &serde_json::Value);

    fn get_server_time(&self, callback: Box<dyn FnOnce(BridgeResult<i64>)>);

    /// `false` while the platform has audio off.
    fn audio_state_changed(&self) -> &Signal<bool>;
    fn pause_state_changed(&self) -> &Signal<bool>;
    /// The id of every message the game sent, itself included.
    fn message_sent(&self) -> &Signal<String>;
}

pub trait DeviceApi {
    fn device_type(&self) -> DeviceType;
    fn os(&self) -> DeviceOs;
    /// The device type as the platform spelled it, for a platform newer than
    /// this crate. [`DeviceApi::device_type`] would report `Desktop` for an id
    /// it does not know.
    fn device_type_text(&self) -> Option<String>;
    /// `None` until the page has told us, which is at once on a real SDK.
    fn orientation(&self) -> Option<DeviceOrientation>;
    /// Insets to keep a HUD clear of notches and platform UI.
    fn safe_area(&self) -> SafeArea;

    fn orientation_state_changed(&self) -> &Signal<Option<DeviceOrientation>>;
    fn screen_size_changed(&self) -> &Signal<Option<ScreenSize>>;
}

pub trait PlayerApi {
    fn is_authorization_supported(&self) -> bool;
    fn is_authorized(&self) -> bool;
    fn is_guest(&self) -> bool;
    fn id(&self) -> Option<String>;
    fn name(&self) -> Option<String>;
    fn photos(&self) -> Vec<String>;
    fn extra(&self) -> ObjectMap;

    fn authorize(
        &self,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
}

pub trait StorageApi {
    /// One entry per key, in the order asked for. A rejected read carries the
    /// reason, such as `STORAGE_QUOTA_EXCEEDED`, not just `Err`.
    fn get(
        &self,
        keys: &[&str],
        try_parse_json: bool,
        callback: Box<dyn FnOnce(BridgeResult<Vec<StorageValue>>)>,
    );
    fn set(&self, pairs: &[(&str, StorageValue)], callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn delete(&self, keys: &[&str], callback: Box<dyn FnOnce(BridgeResult<()>)>);

    /// Fires as a write starts, so it says nothing about whether it landed.
    fn set_performed(&self) -> &Signal<()>;
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
    /// Warm the ad up so the later `show` is not waiting on a network round
    /// trip; the placement falls back to the config's when there is none.
    fn preload_interstitial(&self, placement: Option<&str>);
    fn show_interstitial(&self, placement: Option<&str>);
    fn preload_rewarded(&self, placement: Option<&str>);
    fn show_rewarded(&self, placement: Option<&str>);
    fn show_advanced_banners(&self, placement: Option<&str>);
    fn hide_advanced_banners(&self);

    /// `true` when an ad blocker is in the way.
    fn check_adblock(&self, callback: Box<dyn FnOnce(BridgeResult<bool>)>);

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
    fn is_add_to_favorites_reward_supported(&self) -> bool;
    fn is_add_to_home_screen_reward_supported(&self) -> bool;
    fn is_rate_supported(&self) -> bool;
    fn is_post_reward_supported(&self) -> bool;

    /// `options` is either an entry id from `playgama-bridge-config.json`, or the
    /// content to share.
    fn share(
        &self,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
    fn join_community(
        &self,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
    fn invite_friends(
        &self,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
    /// `payload` is the game's own string for this one post, such as a level, a
    /// seed or a challenge, handed back as `Bridge.platform.payload`.
    fn create_post(
        &self,
        options: Option<&serde_json::Value>,
        payload: Option<&str>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
    fn add_to_favorites(&self, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    /// What the player is owed for adding the game to favourites.
    fn get_add_to_favorites_reward(
        &self,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    );
    fn add_to_home_screen(&self, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    /// What the player is owed for adding the game to their home screen.
    fn get_add_to_home_screen_reward(
        &self,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    );
    fn rate(&self, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn get_post_reward(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
}

pub trait LeaderboardsApi {
    fn leaderboard_type(&self) -> LeaderboardType;
    fn set_score(&self, id: &str, score: f64, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn get_entries(&self, id: &str, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
    fn show_native_popup(&self, id: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>);
}

pub trait PaymentsApi {
    fn is_supported(&self) -> bool;
    fn purchase(
        &self,
        id: &str,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    );
    fn consume_purchase(
        &self,
        id: &str,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    );
    fn get_catalog(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
    fn get_purchases(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
}

pub trait AchievementsApi {
    fn unlock(&self, id: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn get_achievements(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
}

pub trait RemoteConfigApi {
    fn is_supported(&self) -> bool;
    fn set_context(&self, parameters: &serde_json::Value);
    fn get(&self, callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>);
}

pub trait ClipboardApi {
    fn is_supported(&self) -> bool;
    /// The clipboard's text, or the platform's refusal.
    fn read(&self, callback: Box<dyn FnOnce(BridgeResult<String>)>);
    fn write(&self, text: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>);
}

pub trait AnalyticsApi {
    /// Queue one custom event. The SDK batches and ships these itself. A platform
    /// that blocks external calls, and a page served from `file://`, drop them
    /// without saying so.
    fn send(&self, event: &str, data: Option<&serde_json::Value>);
}

pub trait CrossPromoApi {
    fn is_visible(&self) -> bool;
    fn get_games(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>);
    fn show(&self);
    fn hide(&self);

    fn shown(&self) -> &Signal<Option<ObjectMap>>;
}

pub trait TasksApi {
    fn get_tasks(&self, callback: Box<dyn FnOnce(BridgeResult<serde_json::Value>)>);
    fn add_progress(&self, metric: &str, amount: i64, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn claim_reward(&self, task_id: &str, callback: Box<dyn FnOnce(BridgeResult<bool>)>);

    /// The task that was claimed, plus what it paid.
    fn reward_claimed(&self) -> &Signal<Option<ObjectMap>>;
    /// A task set's period ended, so its progress is back to zero.
    fn period_rolled_over(&self) -> &Signal<Option<ObjectMap>>;
}

pub trait DailyRewardsApi {
    fn get_rewards(&self, callback: Box<dyn FnOnce(BridgeResult<serde_json::Value>)>);
    fn get_current_day(&self, callback: Box<dyn FnOnce(BridgeResult<i64>)>);
    /// The whole claimable reward entry from the config, which is `{ id, type,
    /// amount }` plus whatever else the game declared, or `None` when nothing
    /// can be claimed.
    fn get_current_reward(&self, callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>);
    /// `Ok(false)` means there was nothing to claim, which is not an error.
    fn claim_current_reward(&self, callback: Box<dyn FnOnce(BridgeResult<bool>)>);

    fn claimed(&self) -> &Signal<Option<ObjectMap>>;
    fn streak_reset(&self) -> &Signal<Option<ObjectMap>>;
}

pub trait NotificationsApi {
    fn is_supported(&self) -> bool;
    /// Rejects with `NOTIFICATION_INVALID_PARAMETERS` when the object misses an
    /// `id`, `title` or `description`, or types one of them wrongly.
    fn schedule(
        &self,
        notification: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<()>)>,
    );
    fn cancel(&self, id: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>);
    fn cancel_all(&self, callback: Box<dyn FnOnce(BridgeResult<()>)>);
}
