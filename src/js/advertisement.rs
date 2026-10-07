use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::AdvertisementApi;
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::{BannerPosition, BannerState, BridgeResult, InterstitialState, RewardedState};

pub(crate) struct Advertisement {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    banner: Signal<Option<BannerState>>,
    interstitial: Signal<Option<InterstitialState>>,
    rewarded: Signal<Option<RewardedState>>,
    advanced_banners: Signal<Option<BannerState>>,
}

impl Advertisement {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn AdvertisementApi>> {
        let module = modules.get("advertisement")?;
        let ads = Self {
            module: module.clone(),
            in_flight: in_flight.clone(),
            listeners: Listeners::default(),
            banner: Signal::new(),
            interstitial: Signal::new(),
            rewarded: Signal::new(),
            advanced_banners: Signal::new(),
        };
        ads.listeners.listen(
            "banner_state_changed",
            |value| convert::as_string(value).and_then(|s| s.parse().ok()),
            &ads.banner,
        );
        ads.listeners.listen(
            "interstitial_state_changed",
            |value| convert::as_string(value).and_then(|s| s.parse().ok()),
            &ads.interstitial,
        );
        ads.listeners.listen(
            "rewarded_state_changed",
            |value| convert::as_string(value).and_then(|s| s.parse().ok()),
            &ads.rewarded,
        );
        ads.listeners.listen(
            "advanced_banners_state_changed",
            |value| convert::as_string(value).and_then(|s| s.parse().ok()),
            &ads.advanced_banners,
        );
        Some(Rc::new(ads))
    }
}

fn placement_arg(value: Option<&str>) -> JsValue {
    value.map_or(JsValue::NULL, JsValue::from_str)
}

impl AdvertisementApi for Advertisement {
    fn minimum_delay_between_interstitial(&self) -> f64 {
        self.module.f64("minimumDelayBetweenInterstitial")
    }

    fn set_minimum_delay_between_interstitial(&self, seconds: f64) {
        self.module.call1(
            "setMinimumDelayBetweenInterstitial",
            &JsValue::from_f64(seconds),
        );
    }

    fn is_banner_supported(&self) -> bool {
        self.module.bool("isBannerSupported")
    }

    fn banner_state(&self) -> Option<BannerState> {
        self.module.text("bannerState")
    }

    fn is_interstitial_supported(&self) -> bool {
        self.module.bool("isInterstitialSupported")
    }

    fn interstitial_state(&self) -> Option<InterstitialState> {
        self.module.text("interstitialState")
    }

    fn is_rewarded_supported(&self) -> bool {
        self.module.bool("isRewardedSupported")
    }

    fn rewarded_state(&self) -> Option<RewardedState> {
        self.module.text("rewardedState")
    }

    fn rewarded_placement(&self) -> Option<String> {
        self.module.string("rewardedPlacement")
    }

    fn is_advanced_banners_supported(&self) -> bool {
        self.module.bool("isAdvancedBannersSupported")
    }

    fn advanced_banners_state(&self) -> Option<BannerState> {
        self.module.text("advancedBannersState")
    }

    fn show_banner(&self, position: BannerPosition, placement: Option<&str>) {
        self.module.call2(
            "showBanner",
            &JsValue::from_str(position.as_str()),
            &placement_arg(placement),
        );
    }

    fn hide_banner(&self) {
        self.module.call0("hideBanner");
    }

    fn preload_interstitial(&self, placement: Option<&str>) {
        self.module
            .call_opt("preloadInterstitial", Some(placement_arg(placement)));
    }

    fn show_interstitial(&self, placement: Option<&str>) {
        self.module
            .call_opt("showInterstitial", Some(placement_arg(placement)));
    }

    fn preload_rewarded(&self, placement: Option<&str>) {
        self.module
            .call_opt("preloadRewarded", Some(placement_arg(placement)));
    }

    fn show_rewarded(&self, placement: Option<&str>) {
        self.module
            .call_opt("showRewarded", Some(placement_arg(placement)));
    }

    fn show_advanced_banners(&self, placement: Option<&str>) {
        self.module
            .call_opt("showAdvancedBanners", Some(placement_arg(placement)));
    }

    fn hide_advanced_banners(&self) {
        self.module.call0("hideAdvancedBanners");
    }

    fn check_adblock(&self, callback: Box<dyn FnOnce(BridgeResult<bool>)>) {
        let slot = "advertisement.check_adblock";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("checkAdBlock");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|blocked| blocked.as_bool().unwrap_or(false)))
            }),
        );
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
