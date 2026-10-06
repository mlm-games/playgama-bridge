use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::PlatformApi;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::{LaunchSource, PlatformMessage};

pub(crate) struct Platform {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    audio: Signal<bool>,
    pause: Signal<bool>,
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
        };
        platform.listeners.listen(
            &module,
            "audio_state_changed",
            |value| value.as_bool().unwrap_or(false),
            &platform.audio,
        );
        platform.listeners.listen(
            &module,
            "pause_state_changed",
            |value| value.as_bool().unwrap_or(false),
            &platform.pause,
        );
        Some(Rc::new(platform))
    }
}

impl PlatformApi for Platform {
    fn id(&self) -> Option<String> {
        self.module.string("id")
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

    fn get_server_time(&self, callback: Box<dyn FnOnce(i64)>) {
        if !self.in_flight.begin("platform.get_server_time") {
            return;
        }
        let result = self.module.call0("getServerTime");
        settle(
            result,
            &self.in_flight,
            "platform.get_server_time",
            Box::new(move |ok, value| {
                let millis = if ok {
                    value.as_f64().map_or(0, |v| v as i64)
                } else {
                    0
                };
                callback(millis);
            }),
        );
    }

    fn audio_state_changed(&self) -> &Signal<bool> {
        &self.audio
    }

    fn pause_state_changed(&self) -> &Signal<bool> {
        &self.pause
    }
}
