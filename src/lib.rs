//! Rust binding for the Playgama Bridge web SDK, the crate
//! `addons/playgama_bridge` provides to Godot.
//!
//! [`Bridge`] owns the sixteen modules the JS SDK exposes. On the web it wraps
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
//! A promise call hands its callback a [`BridgeResult`], so a refusal carries
//! the platform's own [`ErrorCode`] rather than a bare `false`:
//!
//! ```no_run
//! # use playgama_bridge::{Bridge, ErrorCode};
//! # let bridge = Bridge::new();
//! bridge.notifications.schedule(
//!     Some(&serde_json::json!({
//!         "id": "remind",
//!         "title": "Come back",
//!         "description": "Your run is waiting",
//!     })),
//!     Box::new(|result| match result {
//!         Ok(()) => println!("scheduled"),
//!         Err(error) => match error.code() {
//!             Some(ErrorCode::NotificationInvalidParameters) => println!("bad object"),
//!             _ => println!("{error}"),
//!         },
//!     }),
//! );
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
    AchievementsApi, AdvertisementApi, AnalyticsApi, ClipboardApi, CrossPromoApi, DailyRewardsApi,
    DeviceApi, LeaderboardsApi, NotificationsApi, ObjectMap, PaymentsApi, PlatformApi, PlayerApi,
    RemoteConfigApi, RootApi, SocialApi, StorageApi, TasksApi,
};
pub use inflight::InFlight;
pub use signal::Signal;
pub use types::{
    BannerPosition, BannerState, BridgeError, BridgeResult, DeviceOrientation, DeviceOs,
    DeviceType, ErrorCode, InterstitialState, LaunchSource, LeaderboardType, PlatformId,
    PlatformMessage, PostRewardType, RewardedState, SafeArea, ScreenSize, StorageValue,
    UnknownWireValue,
};

/// Which side of the bridge a [`Bridge`] is talking to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Backend {
    /// The live `window.bridge`.
    Web,
    /// The editor stand-ins.
    Mock,
}

/// The sixteen module handles `Bridge` exposes.
pub struct Bridge {
    /// The root object itself: version, options and the game version.
    pub root: Rc<dyn RootApi>,
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
    pub clipboard: Rc<dyn ClipboardApi>,
    pub analytics: Rc<dyn AnalyticsApi>,
    pub notifications: Rc<dyn NotificationsApi>,
    pub daily_rewards: Rc<dyn DailyRewardsApi>,
    pub tasks: Rc<dyn TasksApi>,
    pub cross_promo: Rc<dyn CrossPromoApi>,
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
            root: web.root,
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
            clipboard: web.clipboard,
            analytics: web.analytics,
            notifications: web.notifications,
            daily_rewards: web.daily_rewards,
            tasks: web.tasks,
            cross_promo: web.cross_promo,
            backend: Backend::Web,
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn with_in_flight(_in_flight: InFlight) -> Self {
        Self::mocks()
    }

    fn mocks() -> Self {
        Self {
            root: mock::root(),
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
            clipboard: mock::clipboard(),
            analytics: mock::analytics(),
            notifications: mock::notifications(),
            daily_rewards: mock::daily_rewards(),
            tasks: mock::tasks(),
            cross_promo: mock::cross_promo(),
            backend: Backend::Mock,
        }
    }

    pub fn backend(&self) -> Backend {
        self.backend
    }

    pub fn is_web(&self) -> bool {
        self.backend == Backend::Web
    }

    /// The SDK version, `"2.3.0"` on the current stable.
    pub fn version(&self) -> Option<String> {
        self.root.version()
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
            .field("version", &self.root.version())
            .field("platform", &self.platform.id())
            .finish_non_exhaustive()
    }
}

/// Read one key as JSON. `Ok(None)` covers "the key is not there" and "the value
/// stored there is not what `T` describes"; an `Err` is the platform's refusal,
/// with its code.
///
/// Reads what [`set_json`] writes, including a bare string or bool, because a
/// stored scalar comes back as text and is parsed here rather than in the SDK.
pub fn get_json<T: serde::de::DeserializeOwned>(
    storage: &dyn StorageApi,
    key: &str,
    callback: impl FnOnce(BridgeResult<Option<T>>) + 'static,
) {
    storage.get(
        &[key],
        true,
        Box::new(move |outcome| {
            let outcome = outcome.map(|values| {
                values.first().and_then(|value| match value {
                    StorageValue::Null => None,
                    other => serde_json::from_value(other.to_json()).ok(),
                })
            });
            callback(outcome);
        }),
    );
}

/// Write one key as JSON text, the format the JS SDK keeps.
pub fn set_json<T: serde::Serialize>(
    storage: &dyn StorageApi,
    key: &str,
    value: &T,
    callback: impl FnOnce(BridgeResult<()>) + 'static,
) {
    let Ok(text) = serde_json::to_string(value) else {
        callback(Err(BridgeError::rejected()));
        return;
    };
    storage.set(&[(key, StorageValue::Str(text))], Box::new(callback));
}

#[cfg(target_arch = "wasm32")]
pub use loader::{
    DEFAULT_ENGINE, LOCAL_BRIDGE_URL, LoadError, LoadOptions, REMOTE_BRIDGE_URL, load,
    set_game_loading_progress,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::rc::Rc;

    #[test]
    fn every_module_is_present_on_both_backends() {
        for bridge in [Bridge::new(), Bridge::mock()] {
            assert!(bridge.platform.platform_id().is_some());
            bridge.analytics.send("noop", None);
            assert!(bridge.version().is_some());
        }
    }

    #[test]
    fn the_mock_refuses_unsupported_modules_with_no_code() {
        let bridge = Bridge::mock();
        let seen = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&seen);
        bridge.payments.get_catalog(Box::new(move |outcome| {
            *slot.borrow_mut() = outcome.err();
        }));
        let error = seen.borrow().clone().expect("answered");
        assert_eq!(error.code(), None, "an unsupported feature has no code");
    }

    #[test]
    fn a_json_write_round_trips_through_the_mock_for_every_json_shape() {
        let bridge = Bridge::mock();
        for (key, value) in [
            ("run", serde_json::json!({"depth": 7})),
            ("level", serde_json::json!(12)),
            ("ratio", serde_json::json!(0.5)),
            ("title", serde_json::json!("mine")),
            ("done", serde_json::json!(true)),
            ("list", serde_json::json!([1, 2])),
        ] {
            set_json(
                &*bridge.storage,
                key,
                &value,
                Box::new(|result: BridgeResult<()>| {
                    assert!(result.is_ok(), "the write did not store");
                }),
            );
            let read = Rc::new(RefCell::new(None));
            let slot = Rc::clone(&read);
            get_json(&*bridge.storage, key, move |outcome| {
                *slot.borrow_mut() = Some(outcome.expect("storage answers"));
            });
            assert_eq!(
                read.borrow().clone().unwrap(),
                Some(value),
                "{key} did not come back"
            );
        }
    }

    #[test]
    fn a_json_read_reports_a_refusal_separately_from_a_missing_key() {
        let bridge = Bridge::mock();
        let missing = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&missing);
        get_json::<serde_json::Value>(&*bridge.storage, "absent", move |outcome| {
            *slot.borrow_mut() = Some(outcome.expect("storage answers"));
        });
        assert_eq!(missing.borrow().clone().unwrap(), None);

        let refused = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&refused);
        bridge.notifications.schedule(
            None,
            Box::new(move |outcome| *slot.borrow_mut() = Some(outcome)),
        );
        assert!(refused.borrow().clone().unwrap().is_err());
    }

    #[test]
    fn a_json_write_round_trips_through_the_mock() {
        let bridge = Bridge::mock();
        #[derive(serde::Serialize, serde::Deserialize, PartialEq, Debug, Clone)]
        struct Run {
            depth: i64,
        }
        set_json(&*bridge.storage, "run", &Run { depth: 7 }, Box::new(|_| {}));
        let read = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&read);
        get_json(&*bridge.storage, "run", move |outcome| {
            *slot.borrow_mut() = Some(outcome.expect("storage answers"));
        });
        assert_eq!(read.borrow().clone().unwrap(), Some(Run { depth: 7 }));
    }
}
