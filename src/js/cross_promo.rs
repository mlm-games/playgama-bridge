use std::rc::Rc;

use crate::api::{CrossPromoApi, ObjectMap};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};

pub(crate) struct CrossPromo {
    module: Module,
    in_flight: InFlight,
}

impl CrossPromo {
    pub(crate) fn attach(modules: &Modules) -> Option<Rc<dyn CrossPromoApi>> {
        Some(Rc::new(Self {
            module: modules.get("crossPromo")?,
            in_flight: InFlight::default(),
        }))
    }
}

impl CrossPromoApi for CrossPromo {
    fn is_visible(&self) -> bool {
        self.module.bool("isVisible")
    }

    fn get_games(&self, callback: Box<dyn FnOnce(bool, Vec<ObjectMap>)>) {
        if !self.in_flight.begin("cross_promo.get_games") {
            return;
        }
        let result = self.module.call0("getGames");
        settle(
            result,
            &self.in_flight,
            "cross_promo.get_games",
            Box::new(move |ok, value| {
                let games = if ok {
                    convert::array_to_vec_of_maps(&value)
                } else {
                    Vec::new()
                };
                callback(ok, games);
            }),
        );
    }

    fn show(&self) {
        self.module.call0("show");
    }

    fn hide(&self) {
        self.module.call0("hide");
    }
}
