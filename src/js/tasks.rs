use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::TasksApi;
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};

pub(crate) struct Tasks {
    module: Module,
    in_flight: InFlight,
}

impl Tasks {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn TasksApi>> {
        Some(Rc::new(Self {
            module: modules.get("tasks")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl TasksApi for Tasks {
    fn get_tasks(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>) {
        if !self.in_flight.begin("tasks.get_tasks") {
            return;
        }
        let result = self.module.call0("getTasks");
        settle(
            result,
            &self.in_flight,
            "tasks.get_tasks",
            Box::new(move |ok, value| {
                // Targets and rewards nest arrays inside objects, so the whole
                // value is converted, not just the top level.
                let tasks = if ok {
                    convert::from_js(&value)
                } else {
                    serde_json::Value::Array(Vec::new())
                };
                callback(ok, tasks);
            }),
        );
    }

    fn add_progress(&self, metric: &str, amount: i64, callback: Box<dyn FnOnce(bool)>) {
        let slot = "tasks.add_progress";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self.module.call2(
            "addProgress",
            &JsValue::from_str(metric),
            &JsValue::from_f64(amount as f64),
        );
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn claim_reward(&self, task_id: &str, callback: Box<dyn FnOnce(bool)>) {
        let slot = "tasks.claim_reward";
        if !self.in_flight.begin(slot) {
            return;
        }
        let result = self
            .module
            .call1("claimReward", &JsValue::from_str(task_id));
        settle(
            result,
            &self.in_flight,
            slot,
            Box::new(move |ok, value| {
                // The promise resolves with whether the claim worked.
                let claimed = value.as_bool().unwrap_or(false);
                callback(ok && claimed);
            }),
        );
    }
}
