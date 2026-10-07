use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::{AchievementsApi, ObjectMap};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::BridgeResult;

pub(crate) struct Achievements {
    module: Module,
    in_flight: InFlight,
}

impl Achievements {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn AchievementsApi>> {
        Some(Rc::new(Self {
            module: modules.get("achievements")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl AchievementsApi for Achievements {
    fn unlock(&self, id: &str, callback: Box<dyn FnOnce(BridgeResult<()>)>) {
        let slot = "achievements.unlock";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call1("unlock", &JsValue::from_str(id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |outcome| callback(outcome.map(|_| ()))),
        );
    }

    fn get_achievements(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>) {
        let slot = "achievements.get_achievements";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call0("getAchievements");
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
