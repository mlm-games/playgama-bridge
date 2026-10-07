use std::rc::Rc;

use crate::api::{DailyRewardsApi, ObjectMap};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::BridgeResult;

pub(crate) struct DailyRewards {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    claimed: Signal<Option<ObjectMap>>,
    streak_reset: Signal<Option<ObjectMap>>,
}

impl DailyRewards {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn DailyRewardsApi>> {
        let module = modules.get("dailyRewards")?;
        let rewards = Self {
            module: module.clone(),
            in_flight: in_flight.clone(),
            listeners: Listeners::default(),
            claimed: Signal::new(),
            streak_reset: Signal::new(),
        };
        rewards.listeners.listen(
            "daily_rewards_claimed",
            convert::optional_object,
            &rewards.claimed,
        );
        rewards.listeners.listen(
            "daily_rewards_streak_reset",
            convert::optional_object,
            &rewards.streak_reset,
        );
        Some(Rc::new(rewards))
    }
}

impl DailyRewardsApi for DailyRewards {
    fn get_rewards(&self, callback: Box<dyn FnOnce(BridgeResult<serde_json::Value>)>) {
        if !self.in_flight.begin("daily_rewards.get_rewards") {
            return;
        }
        let result = self.module.call0("getRewards");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_rewards",
            Box::new(move |outcome| {
                // Targets and rewards nest arrays inside objects, so the whole
                // value is converted, not just the top level.
                callback(outcome.map(|value| convert::from_js(&value)))
            }),
        );
    }

    fn get_current_day(&self, callback: Box<dyn FnOnce(BridgeResult<i64>)>) {
        if !self.in_flight.begin("daily_rewards.get_current_day") {
            return;
        }
        let result = self.module.call0("getCurrentDay");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_current_day",
            Box::new(move |outcome| {
                callback(outcome.map(|value| value.as_f64().map_or(0, |day| day as i64)))
            }),
        );
    }

    fn get_current_reward(&self, callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>) {
        if !self.in_flight.begin("daily_rewards.get_current_reward") {
            return;
        }
        let result = self.module.call0("getCurrentReward");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.get_current_reward",
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::optional_object(&value)))
            }),
        );
    }

    fn claim_current_reward(&self, callback: Box<dyn FnOnce(BridgeResult<bool>)>) {
        if !self.in_flight.begin("daily_rewards.claim_current_reward") {
            return;
        }
        let result = self.module.call0("claimCurrentReward");
        settle(
            result,
            &self.in_flight,
            "daily_rewards.claim_current_reward",
            Box::new(move |outcome| {
                // The promise resolves with whether the claim worked.
                callback(outcome.map(|value| value.as_bool().unwrap_or(false)))
            }),
        );
    }

    fn claimed(&self) -> &Signal<Option<ObjectMap>> {
        &self.claimed
    }

    fn streak_reset(&self) -> &Signal<Option<ObjectMap>> {
        &self.streak_reset
    }
}
