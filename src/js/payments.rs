use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::{ObjectMap, PaymentsApi};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::BridgeResult;

pub(crate) struct Payments {
    module: Module,
    in_flight: InFlight,
}

impl Payments {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn PaymentsApi>> {
        Some(Rc::new(Self {
            module: modules.get("payments")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl PaymentsApi for Payments {
    fn is_supported(&self) -> bool {
        self.module.bool("isSupported")
    }

    fn purchase(
        &self,
        id: &str,
        options: Option<&serde_json::Value>,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    ) {
        let slot = "payments.purchase";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self
            .module
            .call2("purchase", &JsValue::from_str(id), &convert::arg(options));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::optional_object(&value)))
            }),
        );
    }

    fn consume_purchase(
        &self,
        id: &str,
        callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>,
    ) {
        let slot = "payments.consume_purchase";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("consumePurchase", &JsValue::from_str(id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::optional_object(&value)))
            }),
        );
    }

    fn get_catalog(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>) {
        let slot = "payments.get_catalog";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("getCatalog");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::array_to_vec_of_maps(&value)))
            }),
        );
    }

    fn get_purchases(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>) {
        let slot = "payments.get_purchases";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("getPurchases");
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::array_to_vec_of_maps(&value)))
            }),
        );
    }
}
