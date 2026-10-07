use std::rc::Rc;

use crate::api::AnalyticsApi;
use crate::convert;
use crate::js::{Module, Modules};

pub(crate) struct Analytics {
    module: Module,
}

impl Analytics {
    pub(crate) fn attach(modules: &Modules) -> Option<Rc<dyn AnalyticsApi>> {
        Some(Rc::new(Self {
            module: modules.get("analytics")?,
        }))
    }
}

impl AnalyticsApi for Analytics {
    fn send(&self, event: &str, data: Option<&serde_json::Value>) {
        // `send` returns nothing: the SDK queues the event and ships it on its
        // own schedule, so there is no promise to wait on.
        let payload = convert::to_js(&data.cloned().unwrap_or_else(|| serde_json::json!({})));
        self.module
            .call2("send", &wasm_bindgen::JsValue::from_str(event), &payload);
    }
}
