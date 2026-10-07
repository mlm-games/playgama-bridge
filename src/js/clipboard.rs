use std::rc::Rc;

use wasm_bindgen::JsValue;

use crate::api::ClipboardApi;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::BridgeResult;

pub(crate) struct Clipboard {
    module: Module,
    in_flight: InFlight,
}

impl Clipboard {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn ClipboardApi>> {
        Some(Rc::new(Self {
            module: modules.get("clipboard")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl ClipboardApi for Clipboard {
    fn is_supported(&self) -> bool {
        self.module.bool("isSupported")
    }

    fn read(&self, callback: Box<dyn FnOnce(BridgeResult<String>)>) {
        let slot = "clipboard.read";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("read");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|text| text.as_string().unwrap_or_default()))
            }),
        );
    }

    fn write(&self, text: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>) {
        let slot = "clipboard.write";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("write", &JsValue::from_str(text));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| callback(outcome.map(|_| ()))),
        );
    }
}
