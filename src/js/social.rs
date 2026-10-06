use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::{ObjectMap, SocialApi};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};

pub(crate) struct Social {
    module: Module,
    in_flight: InFlight,
}

impl Social {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn SocialApi>> {
        Some(Rc::new(Self {
            module: modules.get("social")?,
            in_flight: in_flight.clone(),
        }))
    }

    fn ok(
        &self,
        slot: &'static str,
        call: impl FnOnce() -> JsValue,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        if !self.in_flight.begin(slot) {
            return;
        }
        settle(
            call(),
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn share_like(
        &self,
        slot: &'static str,
        method: &str,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        self.ok(
            slot,
            || self.module.call_opt(method, Some(convert::arg(options))),
            callback,
        );
    }
}

impl SocialApi for Social {
    fn is_share_supported(&self) -> bool {
        self.module.bool("isShareSupported")
    }

    fn is_join_community_supported(&self) -> bool {
        self.module.bool("isJoinCommunitySupported")
    }

    fn is_invite_friends_supported(&self) -> bool {
        self.module.bool("isInviteFriendsSupported")
    }

    fn is_create_post_supported(&self) -> bool {
        self.module.bool("isCreatePostSupported")
    }

    fn is_add_to_favorites_supported(&self) -> bool {
        self.module.bool("isAddToFavoritesSupported")
    }

    fn is_add_to_home_screen_supported(&self) -> bool {
        self.module.bool("isAddToHomeScreenSupported")
    }

    fn is_rate_supported(&self) -> bool {
        self.module.bool("isRateSupported")
    }

    fn is_post_reward_supported(&self) -> bool {
        self.module.bool("isPostRewardSupported")
    }

    fn share(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        self.share_like("social.share", "share", options, callback);
    }

    fn join_community(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        self.share_like("social.join_community", "joinCommunity", options, callback);
    }

    fn invite_friends(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        self.share_like("social.invite_friends", "inviteFriends", options, callback);
    }

    fn create_post(
        &self,
        options: Option<&serde_json::Value>,
        payload: Option<&str>,
        callback: Box<dyn FnOnce(bool)>,
    ) {
        let slot = "social.create_post";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = match payload {
            Some(text) => self.module.call2(
                "createPost",
                &convert::arg(options),
                &JsValue::from_str(text),
            ),
            None => self
                .module
                .call_opt("createPost", Some(convert::arg(options))),
        };
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn add_to_favorites(&self, callback: Box<dyn FnOnce(bool)>) {
        self.ok(
            "social.add_to_favorites",
            || self.module.call0("addToFavorites"),
            callback,
        );
    }

    fn add_to_home_screen(&self, callback: Box<dyn FnOnce(bool)>) {
        self.ok(
            "social.add_to_home_screen",
            || self.module.call0("addToHomeScreen"),
            callback,
        );
    }

    fn rate(&self, callback: Box<dyn FnOnce(bool)>) {
        self.ok("social.rate", || self.module.call0("rate"), callback);
    }

    fn get_post_reward(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        let slot = "social.get_post_reward";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("getPostReward");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, value| {
                let rewards = if ok {
                    convert::array_to_vec_of_maps(&value)
                } else {
                    Vec::new()
                };
                callback(ok, rewards);
            }),
        );
    }
}
