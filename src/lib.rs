//! Rust binding for the Playgama Bridge web SDK — the crate `addons/playgama_bridge`
//! provides to Godot.
//!
//! [`Bridge`] owns the fourteen modules the JS SDK exposes. On the web it wraps
//! the live `window.bridge`; everywhere else it serves the same editor stand-ins
//! the Godot addon ships, so game code is written once and still builds for the
//! desktop and for headless tests.
//!
//! ```no_run
//! let bridge = playgama_bridge::Bridge::new();
//! bridge
//!     .platform
//!     .send_message(playgama_bridge::PlatformMessage::GameReady);
//! bridge
//!     .advertisement
//!     .rewarded_state_changed()
//!     .connect(|state| println!("rewarded: {state:?}"));
//! ```
//!
//! On the web, [`load`] first pulls `playgama-bridge.js` onto the page and
//! initializes it; it is wasm-only because there is nothing to load otherwise.

mod api;
mod inflight;
mod mock;
mod signal;
mod types;

#[cfg(target_arch = "wasm32")]
mod convert;
#[cfg(target_arch = "wasm32")]
mod js;
#[cfg(target_arch = "wasm32")]
mod loader;

use std::rc::Rc;

pub use api::{
    AchievementsApi, AdvertisementApi, CrossPromoApi, DailyRewardsApi, DeviceApi, LeaderboardsApi,
    NotificationsApi, ObjectMap, PaymentsApi, PlatformApi, PlayerApi, RemoteConfigApi, SocialApi,
    StorageApi, TasksApi,
};
pub use inflight::InFlight;
pub use signal::Signal;
pub use types::{
    BannerPosition, BannerState, DeviceType, InterstitialState, LaunchSource, LeaderboardType,
    PlatformMessage, PostRewardType, RewardedState, StorageValue, UnknownWireValue,
};

/// Which side of the bridge a [`Bridge`] is talking to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// The live `window.bridge`.
    Web,
    /// The editor stand-ins.
    Mock,
}

/// The fourteen module handles `Bridge` exposes.
pub struct Bridge {
    pub platform: Rc<dyn PlatformApi>,
    pub device: Rc<dyn DeviceApi>,
    pub player: Rc<dyn PlayerApi>,
    pub storage: Rc<dyn StorageApi>,
    pub advertisement: Rc<dyn AdvertisementApi>,
    pub social: Rc<dyn SocialApi>,
    pub leaderboards: Rc<dyn LeaderboardsApi>,
    pub payments: Rc<dyn PaymentsApi>,
    pub achievements: Rc<dyn AchievementsApi>,
    pub remote_config: Rc<dyn RemoteConfigApi>,
    pub cross_promo: Rc<dyn CrossPromoApi>,
    pub tasks: Rc<dyn TasksApi>,
    pub daily_rewards: Rc<dyn DailyRewardsApi>,
    pub notifications: Rc<dyn NotificationsApi>,
    backend: Backend,
}

impl Bridge {
    /// The live SDK when the page has one, the editor stand-ins otherwise.
    pub fn new() -> Self {
        Self::with_in_flight(InFlight::default())
    }

    /// Always the editor stand-ins, whatever the page has.
    pub fn mock() -> Self {
        Self::mocks()
    }

    #[cfg(target_arch = "wasm32")]
    fn with_in_flight(in_flight: InFlight) -> Self {
        let Some(web) = js::build(&in_flight) else {
            return Self::mocks();
        };
        Self {
            platform: web.platform,
            device: web.device,
            player: web.player,
            storage: web.storage,
            advertisement: web.advertisement,
            social: web.social,
            leaderboards: web.leaderboards,
            payments: web.payments,
            achievements: web.achievements,
            remote_config: web.remote_config,
            cross_promo: web.cross_promo,
            tasks: web.tasks,
            daily_rewards: web.daily_rewards,
            notifications: web.notifications,
            backend: Backend::Web,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn with_in_flight(_in_flight: InFlight) -> Self {
        Self::mocks()
    }

    fn mocks() -> Self {
        Self {
            platform: mock::platform(),
            device: mock::device(),
            player: mock::player(),
            storage: mock::storage(),
            advertisement: mock::advertisement(),
            social: mock::social(),
            leaderboards: mock::leaderboards(),
            payments: mock::payments(),
            achievements: mock::achievements(),
            remote_config: mock::remote_config(),
            cross_promo: mock::cross_promo(),
            tasks: mock::tasks(),
            daily_rewards: mock::daily_rewards(),
            notifications: mock::notifications(),
            backend: Backend::Mock,
        }
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }

    pub fn is_web(&self) -> bool {
        self.backend == Backend::Web
    }
}

impl Default for Bridge {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Debug for Bridge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Bridge")
            .field("backend", &self.backend)
            .field("platform", &self.platform.id())
            .finish_non_exhaustive()
    }
}

/// Read one key as JSON. `None` covers both "the platform refused" and "the key
/// is not there", which is what the game needs to fall back to local data.
pub fn get_json<T: serde::de::DeserializeOwned>(
    storage: &dyn StorageApi,
    key: &str,
    callback: impl FnOnce(Option<T>) + 'static,
) {
    storage.get(
        &[key],
        true,
        Box::new(move |_, values| {
            let parsed = match values.first() {
                Some(StorageValue::Json(value)) => serde_json::from_value(value.clone()).ok(),
                Some(StorageValue::Int(value)) => {
                    serde_json::from_value(serde_json::Value::from(*value)).ok()
                }
                Some(StorageValue::Float(value)) => serde_json::Number::from_f64(*value)
                    .map(serde_json::Value::Number)
                    .and_then(|number| serde_json::from_value(number).ok()),
                _ => None,
            };
            callback(parsed);
        }),
    );
}

/// Write one key as JSON text, the format the JS SDK keeps.
pub fn set_json<T: serde::Serialize>(
    storage: &dyn StorageApi,
    key: &str,
    value: &T,
    callback: impl FnOnce(bool) + 'static,
) {
    let Ok(text) = serde_json::to_string(value) else {
        callback(false);
        return;
    };
    storage.set(&[(key, StorageValue::Str(text))], Box::new(callback));
}

#[cfg(target_arch = "wasm32")]
pub use loader::{
    DEFAULT_ENGINE, LOCAL_BRIDGE_URL, LoadError, LoadOptions, REMOTE_BRIDGE_URL, load,
    set_game_loading_progress,
};
