use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::PlatformApi;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::{BridgeResult, LaunchSource, PlatformId, PlatformMessage};

pub(crate) struct Platform {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    audio: Signal<bool>,
    pause: Signal<bool>,
    sent: Signal<String>,
}

impl Platform {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn PlatformApi>> {
        let module = modules.get("platform")?;
        let platform = Self {
            module: module.clone(),
            in_flight: in_flight.clone(),
            listeners: Listeners::default(),
            audio: Signal::new(),
            pause: Signal::new(),
            sent: Signal::new(),
        };
        platform.listeners.listen(
            "audio_state_changed",
            |value| value.as_bool().unwrap_or(false),
            &platform.audio,
        );
        platform.listeners.listen(
            "pause_state_changed",
            |value| value.as_bool().unwrap_or(false),
            &platform.pause,
        );
        platform.listeners.listen(
            "platform_message_sent",
            |value| value.as_string().unwrap_or_default(),
            &platform.sent,
        );
        Some(Rc::new(platform))
    }
}

impl PlatformApi for Platform {
    fn id(&self) -> Option<String> {
        self.module.string("id")
    }

    fn platform_id(&self) -> Option<PlatformId> {
        self.module.text("id")
    }

    fn sdk(&self) -> Option<String> {
        self.module.string("sdk")
    }

    fn payload(&self) -> Option<String> {
        self.module.string("payload")
    }

    fn language(&self) -> Option<String> {
        self.module.string("language")
    }

    fn tld(&self) -> Option<String> {
        self.module.string("tld")
    }

    fn launch_source(&self) -> Option<LaunchSource> {
        self.module.text("launchSource")
    }

    fn data(&self) -> crate::api::ObjectMap {
        self.module.map("data")
    }

    fn is_audio_enabled(&self) -> bool {
        self.module.bool("isAudioEnabled")
    }

    fn is_paused(&self) -> bool {
        self.module.bool("isPaused")
    }

    fn is_external_calls_supported(&self) -> bool {
        self.module.bool("isExternalCallsSupported")
    }

    fn is_external_links_allowed(&self) -> bool {
        self.module.bool("isExternalLinksAllowed")
    }

    fn send_message(&self, message: PlatformMessage) {
        self.module
            .call1("sendMessage", &JsValue::from_str(message.as_str()));
    }

    fn send_message_with(&self, message: PlatformMessage, options: &serde_json::Value) {
        self.module.call2(
            "sendMessage",
            &JsValue::from_str(message.as_str()),
            &crate::convert::to_js(options),
        );
    }

    fn send_custom_message(&self, id: &str) {
        self.module
            .call1("sendCustomMessage", &JsValue::from_str(id));
    }

    fn send_custom_message_with(&self, id: &str, options: &serde_json::Value) {
        self.module.call2(
            "sendCustomMessage",
            &JsValue::from_str(id),
            &crate::convert::to_js(options),
        );
    }

    fn get_server_time(&self, callback: Box<dyn FnOnce(BridgeResult<i64>)>) {
        let slot = "platform.get_server_time";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("getServerTime");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|value| value.as_f64().map_or(0, |millis| millis as i64)))
            }),
        );
    }

    fn audio_state_changed(&self) -> &Signal<bool> {
        &self.audio
    }

    fn pause_state_changed(&self) -> &Signal<bool> {
        &self.pause
    }

    fn message_sent(&self) -> &Signal<String> {
        &self.sent
    }
}
