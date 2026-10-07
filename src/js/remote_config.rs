use std::rc::Rc;

use crate::api::{ObjectMap, RemoteConfigApi};
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::BridgeResult;

pub(crate) struct RemoteConfig {
    module: Module,
    in_flight: InFlight,
}

impl RemoteConfig {
    pub(crate) fn attach(
        modules: &Modules,
        in_flight: &InFlight,
    ) -> Option<Rc<dyn RemoteConfigApi>> {
        Some(Rc::new(Self {
            module: modules.get("remoteConfig")?,
            in_flight: in_flight.clone(),
        }))
    }
}

impl RemoteConfigApi for RemoteConfig {
    fn is_supported(&self) -> bool {
        self.module.bool("isSupported")
    }

    fn set_context(&self, parameters: &serde_json::Value) {
        self.module.call1("setContext", &convert::to_js(parameters));
    }

    fn get(&self, callback: Box<dyn FnOnce(BridgeResult<Option<ObjectMap>>)>) {
        if !self.in_flight.begin("remote_config.get") {
            return;
        }
        let result = self.module.call0("get");
        settle(
            result,
            &self.in_flight,
            "remote_config.get",
            Box::new(move |outcome| {
                // A platform with no remote config answers with the raw value;
                // only a real object is a set of parameters.
                callback(outcome.map(|value| convert::optional_object(&value)))
            }),
        );
    }
}
