use std::rc::Rc;

use crate::api::{CrossPromoApi, ObjectMap};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Listeners, Module, Modules, settle};
use crate::signal::Signal;
use crate::types::BridgeResult;

pub(crate) struct CrossPromo {
    module: Module,
    in_flight: InFlight,
    listeners: Listeners,
    shown: Signal<Option<ObjectMap>>,
}

impl CrossPromo {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn CrossPromoApi>> {
        let module = modules.get("crossPromo")?;
        let promo = Self {
            module: module.clone(),
            in_flight: in_flight.clone(),
            listeners: Listeners::default(),
            shown: Signal::new(),
        };
        // The payload is `{ source, games }`; an event with nothing behind it
        // is still worth waking a listener for.
        promo
            .listeners
            .listen("cross_promo_shown", convert::optional_object, &promo.shown);
        Some(Rc::new(promo))
    }
}

impl CrossPromoApi for CrossPromo {
    fn is_visible(&self) -> bool {
        self.module.bool("isVisible")
    }

    fn get_games(&self, callback: Box<dyn FnOnce(BridgeResult<Vec<ObjectMap>>)>) {
        if !self.in_flight.begin("cross_promo.get_games") {
            return;
        }
        let result = self.module.call0("getGames");
        settle(
            result,
            &self.in_flight,
            "cross_promo.get_games",
            Box::new(move |outcome| {
                callback(outcome.map(|value| convert::array_to_vec_of_maps(&value)))
            }),
        );
    }

    fn show(&self) {
        self.module.call0("show");
    }

    fn hide(&self) {
        self.module.call0("hide");
    }

    fn shown(&self) -> &Signal<Option<ObjectMap>> {
        &self.shown
    }
}
