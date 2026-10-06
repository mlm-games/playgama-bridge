use std::rc::Rc;

use crate::api::{ObjectMap, PlayerApi};
use crate::inflight::InFlight;
use crate::js::{Module, Modules, convert, settle};

pub(crate) struct Player {
    module: Module,
    in_flight: InFlight,
}

impl Player {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn PlayerApi>> {
        Some(Rc::new(Self {
            module: modules.get("player")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl PlayerApi for Player {
    fn is_authorization_supported(&self) -> bool {
        self.module.bool("isAuthorizationSupported")
    }

    fn is_authorized(&self) -> bool {
        self.module.bool("isAuthorized")
    }

    fn is_guest(&self) -> bool {
        self.module.bool("isGuest")
    }

    fn id(&self) -> Option<String> {
        self.module.string("id")
    }

    fn name(&self) -> Option<String> {
        self.module.string("name")
    }

    fn photos(&self) -> Vec<String> {
        self.module
            .get("photos")
            .map(|photos| {
                convert::array_to_vec(&photos)
                    .iter()
                    .filter_map(convert::as_string)
                    .collect()
            })
            .unwrap_or_default()
    }

    fn extra(&self) -> ObjectMap {
        self.module.map("extra")
    }

    fn authorize(&self, options: Option<&serde_json::Value>, callback: Box<dyn FnOnce(bool)>) {
        if !self.in_flight.begin("player.authorize") {
            return;
        }
        let result = self
            .module
            .call_opt("authorize", Some(convert::arg(options)));
        settle(
            result,
            &self.in_flight,
            "player.authorize",
            Box::new(move |ok, _| callback(ok)),
        );
    }
}
