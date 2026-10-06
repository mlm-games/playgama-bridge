//! The editor stand-ins `*_editor_mock.gd` provides, so the same game code runs
//! on the desktop and in tests.
//!
//! Every unsupported module answers `false` immediately, exactly like the
//! Godot mocks. Advertisement, player and storage carry the little state the
//! Godot mocks carry: ads run their full state sequence synchronously, storage
//! keeps the text format the JS SDK writes.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::api::*;
use crate::signal::Signal;
use crate::types::*;

pub(crate) struct Platform {
    audio: Signal<bool>,
    pause: Signal<bool>,
}

impl PlatformApi for Platform {
    fn id(&self) -> Option<String> {
        Some("mock".to_string())
    }

    fn payload(&self) -> Option<String> {
        None
    }

    fn language(&self) -> Option<String> {
        Some("en".to_string())
    }

    fn tld(&self) -> Option<String> {
        None
    }

    fn launch_source(&self) -> Option<LaunchSource> {
        None
    }

    fn data(&self) -> ObjectMap {
        ObjectMap::new()
    }

    fn is_audio_enabled(&self) -> bool {
        true
    }

    fn is_external_calls_supported(&self) -> bool {
        true
    }

    fn is_external_links_allowed(&self) -> bool {
        true
    }

    fn send_message(&self, _message: PlatformMessage) {}
    fn send_message_with(&self, _message: PlatformMessage, _options: &serde_json::Value) {}
    fn send_custom_message(&self, _id: &str) {}
    fn send_custom_message_with(&self, _id: &str, _options: &serde_json::Value) {}

    fn get_server_time(&self, callback: Box<dyn FnOnce(i64)>) {
        let millis = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_millis() as i64);
        callback(millis);
    }

    fn audio_state_changed(&self) -> &Signal<bool> {
        &self.audio
    }

    fn pause_state_changed(&self) -> &Signal<bool> {
        &self.pause
    }
}

pub(crate) fn platform() -> Rc<dyn PlatformApi> {
    Rc::new(Platform {
        audio: Signal::new(),
        pause: Signal::new(),
    })
}

pub(crate) struct Device;

impl DeviceApi for Device {
    fn device_type(&self) -> DeviceType {
        DeviceType::Desktop
    }
}

pub(crate) fn device() -> Rc<dyn DeviceApi> {
    Rc::new(Device)
}

pub(crate) struct Player;

impl PlayerApi for Player {
    fn is_authorization_supported(&self) -> bool {
        false
    }

    fn is_authorized(&self) -> bool {
        false
    }

    fn is_guest(&self) -> bool {
        true
    }

    fn id(&self) -> Option<String> {
        None
    }

    fn name(&self) -> Option<String> {
        None
    }

    fn photos(&self) -> Vec<String> {
        Vec::new()
    }

    fn extra(&self) -> ObjectMap {
        ObjectMap::new()
    }

    fn authorize(&self, _options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }
}

pub(crate) fn player() -> Rc<dyn PlayerApi> {
    Rc::new(Player)
}

/// The mock's stand-in for the platform's cloud storage. The Godot mock writes
/// `user://<key>.save`; text in a map keeps the same format without touching
/// the player's disk.
#[derive(Clone, Default)]
pub(crate) struct MockStore(Rc<RefCell<HashMap<String, String>>>);

impl MockStore {
    fn read(&self, key: &str) -> StorageValue {
        match self.0.borrow().get(key) {
            // A missing file and an empty one both read back as null.
            None => StorageValue::Null,
            Some(text) if text.is_empty() => StorageValue::Null,
            Some(text) => StorageValue::Str(text.clone()),
        }
    }
}

pub(crate) struct Storage {
    store: MockStore,
}

impl StorageApi for Storage {
    fn get(
        &self,
        keys: &[&str],
        try_parse_json: bool,
        callback: Box<dyn FnOnce(bool, Vec<StorageValue>)>,
    ) {
        let values = keys
            .iter()
            .map(|key| match self.store.read(key) {
                StorageValue::Str(text) => StorageValue::deserialize(&text, try_parse_json),
                other => other,
            })
            .collect();
        callback(true, values);
    }

    fn set(&self, pairs: &[(&str, StorageValue)], callback: Box<dyn FnOnce(bool)>) {
        for (key, value) in pairs {
            self.store
                .0
                .borrow_mut()
                .insert((*key).to_string(), value.serialize());
        }
        callback(true);
    }

    fn delete(&self, keys: &[&str], callback: Box<dyn FnOnce(bool)>) {
        for key in keys {
            self.store.0.borrow_mut().remove(*key);
        }
        callback(true);
    }
}

pub(crate) fn storage() -> Rc<dyn StorageApi> {
    Rc::new(Storage {
        store: MockStore::default(),
    })
}

pub(crate) struct Advertisement {
    minimum_delay: Cell<f64>,
    banner_state: RefCell<Option<BannerState>>,
    interstitial_state: RefCell<Option<InterstitialState>>,
    rewarded_state: RefCell<Option<RewardedState>>,
    rewarded_placement: RefCell<Option<String>>,
    banner: Signal<Option<BannerState>>,
    interstitial: Signal<Option<InterstitialState>>,
    rewarded: Signal<Option<RewardedState>>,
    advanced_banners: Signal<Option<BannerState>>,
}

impl Advertisement {
    fn set_banner_state(&self, state: BannerState) {
        *self.banner_state.borrow_mut() = Some(state);
        self.banner.emit(Some(state));
    }

    fn set_interstitial_state(&self, state: InterstitialState) {
        *self.interstitial_state.borrow_mut() = Some(state);
        self.interstitial.emit(Some(state));
    }

    fn set_rewarded_state(&self, state: RewardedState) {
        *self.rewarded_state.borrow_mut() = Some(state);
        self.rewarded.emit(Some(state));
    }
}

impl AdvertisementApi for Advertisement {
    fn minimum_delay_between_interstitial(&self) -> f64 {
        self.minimum_delay.get()
    }

    fn set_minimum_delay_between_interstitial(&self, seconds: f64) {
        self.minimum_delay.set(seconds);
    }

    fn is_banner_supported(&self) -> bool {
        true
    }

    fn banner_state(&self) -> Option<BannerState> {
        *self.banner_state.borrow()
    }

    fn is_interstitial_supported(&self) -> bool {
        true
    }

    fn interstitial_state(&self) -> Option<InterstitialState> {
        *self.interstitial_state.borrow()
    }

    fn is_rewarded_supported(&self) -> bool {
        true
    }

    fn rewarded_state(&self) -> Option<RewardedState> {
        *self.rewarded_state.borrow()
    }

    fn rewarded_placement(&self) -> Option<String> {
        self.rewarded_placement.borrow().clone()
    }

    fn is_advanced_banners_supported(&self) -> bool {
        false
    }

    fn advanced_banners_state(&self) -> Option<BannerState> {
        None
    }

    fn show_banner(&self, _position: BannerPosition, _placement: Option<&str>) {
        self.set_banner_state(BannerState::Loading);
        self.set_banner_state(BannerState::Shown);
    }

    fn hide_banner(&self) {
        self.set_banner_state(BannerState::Hidden);
    }

    fn show_interstitial(&self, _placement: Option<&str>) {
        self.set_interstitial_state(InterstitialState::Loading);
        self.set_interstitial_state(InterstitialState::Opened);
        self.set_interstitial_state(InterstitialState::Closed);
    }

    fn show_rewarded(&self, placement: Option<&str>) {
        *self.rewarded_placement.borrow_mut() = placement.map(str::to_string);
        self.set_rewarded_state(RewardedState::Loading);
        self.set_rewarded_state(RewardedState::Opened);
        self.set_rewarded_state(RewardedState::Rewarded);
        self.set_rewarded_state(RewardedState::Closed);
    }

    fn show_advanced_banners(&self, _placement: Option<&str>) {}

    fn hide_advanced_banners(&self) {}

    fn check_adblock(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn banner_state_changed(&self) -> &Signal<Option<BannerState>> {
        &self.banner
    }

    fn interstitial_state_changed(&self) -> &Signal<Option<InterstitialState>> {
        &self.interstitial
    }

    fn rewarded_state_changed(&self) -> &Signal<Option<RewardedState>> {
        &self.rewarded
    }

    fn advanced_banners_state_changed(&self) -> &Signal<Option<BannerState>> {
        &self.advanced_banners
    }
}

pub(crate) fn advertisement() -> Rc<dyn AdvertisementApi> {
    Rc::new(Advertisement {
        minimum_delay: Cell::new(60.0),
        banner_state: RefCell::new(Some(BannerState::Hidden)),
        interstitial_state: RefCell::new(Some(InterstitialState::Closed)),
        rewarded_state: RefCell::new(Some(RewardedState::Closed)),
        rewarded_placement: RefCell::new(None),
        banner: Signal::new(),
        interstitial: Signal::new(),
        rewarded: Signal::new(),
        advanced_banners: Signal::new(),
    })
}

pub(crate) struct Social;

impl SocialApi for Social {
    fn is_share_supported(&self) -> bool {
        false
    }

    fn is_join_community_supported(&self) -> bool {
        false
    }

    fn is_invite_friends_supported(&self) -> bool {
        false
    }

    fn is_create_post_supported(&self) -> bool {
        false
    }

    fn is_add_to_favorites_supported(&self) -> bool {
        false
    }

    fn is_add_to_home_screen_supported(&self) -> bool {
        false
    }

    fn is_rate_supported(&self) -> bool {
        false
    }

    fn is_post_reward_supported(&self) -> bool {
        false
    }

    fn share(&self, _options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn join_community(
        &self,
        _options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        callback(false);
    }

    fn invite_friends(
        &self,
        _options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        callback(false);
    }

    fn create_post(
        &self,
        _options: Option<&serde_json::Value>,
        _payload: Option<&str>,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        callback(false);
    }

    fn add_to_favorites(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn add_to_home_screen(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn rate(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn get_post_reward(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(false, Vec::new());
    }
}

pub(crate) fn social() -> Rc<dyn SocialApi> {
    Rc::new(Social)
}

pub(crate) struct Leaderboards;

impl LeaderboardsApi for Leaderboards {
    fn leaderboard_type(&self) -> LeaderboardType {
        LeaderboardType::NotAvailable
    }

    fn set_score(&self, _id: &str, _score: f64, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn get_entries(&self, _id: &str, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(false, Vec::new());
    }

    fn show_native_popup(&self, _id: &str, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }
}

pub(crate) fn leaderboards() -> Rc<dyn LeaderboardsApi> {
    Rc::new(Leaderboards)
}

pub(crate) struct Payments;

impl PaymentsApi for Payments {
    fn is_supported(&self) -> bool {
        false
    }

    fn purchase(
        &self,
        _id: &str,
        _options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>,
    ) {
        callback(false, None);
    }

    fn consume_purchase(&self, _id: &str, callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>) {
        callback(false, None);
    }

    fn get_catalog(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(false, Vec::new());
    }

    fn get_purchases(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(false, Vec::new());
    }
}

pub(crate) fn payments() -> Rc<dyn PaymentsApi> {
    Rc::new(Payments)
}

pub(crate) struct Achievements;

impl AchievementsApi for Achievements {
    fn unlock(&self, _id: &str, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn get_achievements(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(false, Vec::new());
    }
}

pub(crate) fn achievements() -> Rc<dyn AchievementsApi> {
    Rc::new(Achievements)
}

pub(crate) struct RemoteConfig;

impl RemoteConfigApi for RemoteConfig {
    fn is_supported(&self) -> bool {
        false
    }

    fn set_context(&self, _parameters: &serde_json::Value) {}

    fn get(&self, callback: Box<dyn FnOnce(bool, Option<ObjectMap>)>) {
        callback(false, None);
    }
}

pub(crate) fn remote_config() -> Rc<dyn RemoteConfigApi> {
    Rc::new(RemoteConfig)
}

pub(crate) struct CrossPromo;

impl CrossPromoApi for CrossPromo {
    fn is_visible(&self) -> bool {
        false
    }

    fn get_games(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        callback(true, Vec::new());
    }

    fn show(&self) {}

    fn hide(&self) {}
}

pub(crate) fn cross_promo() -> Rc<dyn CrossPromoApi> {
    Rc::new(CrossPromo)
}

pub(crate) struct Tasks;

impl TasksApi for Tasks {
    fn get_tasks(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>) {
        callback(false, serde_json::Value::Array(Vec::new()));
    }

    fn add_progress(&self, _metric: &str, _amount: i64, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn claim_reward(&self, _task_id: &str, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }
}

pub(crate) fn tasks() -> Rc<dyn TasksApi> {
    Rc::new(Tasks)
}

pub(crate) struct DailyRewards;

impl DailyRewardsApi for DailyRewards {
    fn get_rewards(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>) {
        callback(false, serde_json::Value::Array(Vec::new()));
    }

    fn get_current_day(&self, callback: Box<dyn FnOnce(bool, i64)>) {
        callback(false, 0);
    }

    fn get_current_reward(&self, callback: Box<dyn FnOnce(bool, Option<String>)>) {
        callback(false, None);
    }

    fn claim_current_reward(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }
}

pub(crate) fn daily_rewards() -> Rc<dyn DailyRewardsApi> {
    Rc::new(DailyRewards)
}

pub(crate) struct Notifications;

impl NotificationsApi for Notifications {
    fn is_supported(&self) -> bool {
        false
    }

    fn schedule(&self, _notification: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn cancel(&self, _id: &str, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }

    fn cancel_all(&self, callback: Box<dyn FnOnce(bool)>) {
        callback(false);
    }
}

pub(crate) fn notifications() -> Rc<dyn NotificationsApi> {
    Rc::new(Notifications)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn the_ad_mock_runs_its_whole_state_sequence() {
        let ads = advertisement();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        ads.interstitial_state_changed()
            .connect(move |state| log.borrow_mut().push(state));
        ads.show_interstitial(None);
        assert_eq!(
            *seen.borrow(),
            vec![
                Some(InterstitialState::Loading),
                Some(InterstitialState::Opened),
                Some(InterstitialState::Closed),
            ]
        );
        assert_eq!(ads.interstitial_state(), Some(InterstitialState::Closed));
    }

    #[test]
    fn the_ad_mock_grants_the_reward_before_it_closes() {
        let ads = advertisement();
        let granted = Rc::new(Cell::new(false));
        let seen = Rc::clone(&granted);
        ads.rewarded_state_changed().connect(move |state| {
            if state == Some(RewardedState::Rewarded) {
                seen.set(true);
            }
        });
        ads.show_rewarded(Some("revive"));
        assert!(granted.get());
        assert_eq!(ads.rewarded_placement().as_deref(), Some("revive"));
    }

    #[test]
    fn the_storage_mock_keeps_the_js_text_format() {
        let storage = storage();
        storage.set(
            &[("run", StorageValue::Json(serde_json::json!({"depth": 3})))],
            Box::new(|_| {}),
        );
        let read = Rc::new(RefCell::new(None));
        let slot = Rc::clone(&read);
        storage.get(
            &["run"],
            true,
            Box::new(move |ok, values| {
                *slot.borrow_mut() = Some((ok, values));
            }),
        );
        let (ok, values) = read.borrow().clone().expect("read back");
        assert!(ok);
        assert_eq!(
            values[0],
            StorageValue::Json(serde_json::json!({"depth": 3}))
        );
    }

    #[test]
    fn a_missing_key_reads_back_as_null() {
        let storage = storage();
        let read = Rc::new(RefCell::new(Vec::new()));
        let slot = Rc::clone(&read);
        storage.get(
            &["nope"],
            true,
            Box::new(move |_, values| {
                *slot.borrow_mut() = values;
            }),
        );
        assert_eq!(read.borrow()[0], StorageValue::Null);
    }

    #[test]
    fn unsupported_modules_reject_right_away() {
        let social = social();
        let ok = Rc::new(Cell::new(true));
        let slot = Rc::clone(&ok);
        social.share(None, Box::new(move |granted| slot.set(granted)));
        assert!(!ok.get());
        assert_eq!(platform().id().as_deref(), Some("mock"));
        assert_eq!(device().device_type(), DeviceType::Desktop);
        assert!(!player().is_authorized());
        assert_eq!(
            leaderboards().leaderboard_type(),
            LeaderboardType::NotAvailable
        );
    }
}
