use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::{LeaderboardsApi, ObjectMap};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::LeaderboardType;

pub(crate) struct Leaderboards {
    module: Module,
    in_flight: InFlight,
}

impl Leaderboards {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn LeaderboardsApi>> {
        Some(Rc::new(Self {
            module: modules.get("leaderboards")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl LeaderboardsApi for Leaderboards {
    fn leaderboard_type(&self) -> LeaderboardType {
        self.module
            .text("type")
            .unwrap_or(LeaderboardType::NotAvailable)
    }

    fn set_score(&self, id: &str, score: f64, callback: Box<dyn FnOnce(bool)>) {
        let slot = "leaderboards.set_score";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call2(
            "setScore",
            &JsValue::from_str(id),
            &JsValue::from_f64(score),
        );
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn get_entries(&self, id: &str, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        let slot = "leaderboards.get_entries";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("getEntries", &JsValue::from_str(id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, value| {
                let entries = if ok {
                    convert::array_to_vec_of_maps(&value)
                } else {
                    Vec::new()
                };
                callback(ok, entries);
            }),
        );
    }

    fn show_native_popup(&self, id: &str, callback: Box<dyn FnOnce(bool)>) {
        let slot = "leaderboards.show_native_popup";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("showNativePopup", &JsValue::from_str(id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }
}
