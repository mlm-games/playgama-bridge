use std::rc::Rc;

use js_sys::Reflect;
use wasm_bindgen::JsValue;

use crate::api::{ObjectMap, RootApi};
use crate::convert;
use crate::js::global_bridge;

pub(crate) struct Root {
    object: JsValue,
}

impl Root {
    pub(crate) fn attach() -> Rc<dyn RootApi> {
        Rc::new(Self {
            object: global_bridge().unwrap_or(JsValue::UNDEFINED),
        })
    }
}

impl RootApi for Root {
    fn version(&self) -> Option<String> {
        self.string("version")
    }

    fn is_initialized(&self) -> bool {
        self.bool("isInitialized")
    }

    fn options(&self) -> ObjectMap {
        self.get("options")
            .map_or_else(ObjectMap::new, |options| convert::object_to_map(&options))
    }

    fn set_game_version(&self, version: &str) {
        let _ = Reflect::set(
            &self.object,
            &JsValue::from_str("gameVersion"),
            &JsValue::from_str(version),
        );
    }
}

impl Root {
    fn get(&self, key: &str) -> Option<JsValue> {
        let value = Reflect::get(&self.object, &JsValue::from_str(key)).ok()?;
        (!value.is_undefined() && !value.is_null()).then_some(value)
    }

    fn string(&self, key: &str) -> Option<String> {
        self.get(key)?.as_string()
    }

    fn bool(&self, key: &str) -> bool {
        self.get(key)
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
    }
}
