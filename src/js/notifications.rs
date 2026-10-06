use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::NotificationsApi;
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};

pub(crate) struct Notifications {
    module: Module,
    in_flight: InFlight,
}

impl Notifications {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn NotificationsApi>> {
        Some(Rc::new(Self {
            module: modules.get("notifications")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl NotificationsApi for Notifications {
    fn is_supported(&self) -> bool {
        self.module.bool("isSupported")
    }

    fn schedule(&self, notification: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        let slot = "notifications.schedule";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self
            .module
            .call_opt("schedule", Some(convert::arg(notification)));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn cancel(&self, id: &str, callback: Box<dyn FnOnce(bool)>) {
        let slot = "notifications.cancel";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("cancel", &JsValue::from_str(id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn cancel_all(&self, callback: Box<dyn FnOnce(bool)>) {
        if !self.in_flight.begin("notifications.cancel_all") {
            return;
        }
        let result = self.module.call0("cancelAll");
        settle(
            result,
            &self.in_flight,
            "notifications.cancel_all",
            Box::new(move |ok, _| callback(ok)),
        );
    }
}
