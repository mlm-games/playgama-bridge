use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::{ObjectMap, TasksApi};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::BridgeResult;

pub(crate) struct Tasks {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    claimed: Signal<Option<ObjectMap>>,
    rolled_over: Signal<Option<ObjectMap>>,
}

impl Tasks {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn TasksApi>> {
        let module = modules.get("tasks")?;
        let tasks = Self {
            module: module.clone(),
            in_flight: in_flight.clone(),
            listeners: Listeners::default(),
            claimed: Signal::new(),
            rolled_over: Signal::new(),
        };
        tasks.listeners.listen(
            "tasks_reward_claimed",
            convert::optional_object,
            &tasks.claimed,
        );
        tasks.listeners.listen(
            "tasks_period_rolled_over",
            convert::optional_object,
            &tasks.rolled_over,
        );
        Some(Rc::new(tasks))
    }
}

impl TasksApi for Tasks {
    fn get_tasks(&self, callback: Box<dyn FnOnce(BridgeResult<serde_json::Value>)>) {
        if !self.in_flight.begin("tasks.get_tasks") {
            return;
        }
        let result = self.module.call0("getTasks");
        settle(
            result,
            &self.in_flight,
            "tasks.get_tasks",
            Box::new(move |outcome| {
                // Targets and rewards nest arrays inside objects, so the whole
                // value is converted, not just the top level.
                callback(outcome.map(|value| convert::from_js(&value)))
            }),
        );
    }

    fn add_progress(&self, metric: &str, amount: i64, callback: Box<dyn FnOnce(BridgeResult<()>)>) {
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
            Box::new(move |outcome| callback(outcome.map(|_| ()))),
        );
    }

    fn claim_reward(&self, task_id: &str, callback: Box<dyn FnOnce(BridgeResult<bool>)>) {
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
            Box::new(move |outcome| {
                // The promise resolves with whether the claim worked.
                callback(outcome.map(|value| value.as_bool().unwrap_or(false)))
            }),
        );
    }

    fn reward_claimed(&self) -> &Signal<Option<ObjectMap>> {
        &self.claimed
    }

    fn period_rolled_over(&self) -> &Signal<Option<ObjectMap>> {
        &self.rolled_over
    }
}
