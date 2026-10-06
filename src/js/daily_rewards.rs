use std::rc::Rc;

use crate::api::DailyRewardsApi;
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};

pub(crate) struct DailyRewards {
    module: Module,
    in_flight: InFlight,
}

impl DailyRewards {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn DailyRewardsApi>> {
        Some(Rc::new(Self {
            module: modules.get("dailyRewards")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl DailyRewardsApi for DailyRewards {
    fn get_rewards(&self, callback: Box<dyn FnOnce(bool, serde_json::Value)>) {
        if !self.in_flight.begin("daily_rewards.get_rewards") {
            return;
        }
        let result = self.module.call0("getRewards");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_rewards",
            Box::new(move |ok, value| {
                let rewards = if ok {
                    convert::from_js(&value)
                } else {
                    serde_json::Value::Array(Vec::new())
                };
                callback(ok, rewards);
            }),
        );
    }

    fn get_current_day(&self, callback: Box<dyn FnOnce(bool, i64)>) {
        if !self.in_flight.begin("daily_rewards.get_current_day") {
            return;
        }
        let result = self.module.call0("getCurrentDay");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_current_day",
            Box::new(move |ok, value| {
                let day = if ok {
                    value.as_f64().map_or(0, |v| v as i64)
                } else {
                    0
                };
                callback(ok, day);
            }),
        );
    }

    fn get_current_reward(&self, callback: Box<dyn FnOnce(bool, Option<String>)>) {
        if !self.in_flight.begin("daily_rewards.get_current_reward") {
            return;
        }
        let result = self.module.call0("getCurrentReward");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_current_reward",
            Box::new(move |ok, value| callback(ok, convert::as_string(&value))),
        );
    }

    fn claim_current_reward(&self, callback: Box<dyn FnOnce(bool)>) {
        if !self.in_flight.begin("daily_rewards.claim_current_reward") {
            return;
        }
        let result = self.module.call0("claimCurrentReward");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.claim_current_reward",
            Box::new(move |ok, value| {
                // The promise resolves with whether the claim worked.
                callback(ok && value.as_bool().unwrap_or(false));
            }),
        );
    }
}
