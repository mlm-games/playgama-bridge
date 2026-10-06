use std::rc::Rc;
use wasm_bindgen::JsValue;

use crate::api::StorageApi;
use crate::convert;
use crate::inflight::InFlight;
use crate::js::{Module, Modules, settle};
use crate::types::StorageValue;

pub(crate) struct Storage {
    module: Module,
    in_flight: InFlight,
}

impl Storage {
    pub(crate) fn attach(modules: &Modules, in_flight: &InFlight) -> Option<Rc<dyn StorageApi>> {
        Some(Rc::new(Self {
            module: modules.get("storage")?,
            in_flight: in_flight.clone(),
        }))
    }
}

/// One stored entry. The JS SDK hands back whatever it kept, so a JSON string
/// stays a string until `try_parse_json` turns it into JSON.
fn from_js(value: &JsValue, try_parse_json: bool) -> StorageValue {
    if value.is_null() || value.is_undefined() {
        return StorageValue::Null;
    }
    match convert::as_string(value) {
        Some(text) => StorageValue::deserialize(&text, try_parse_json),
        None => StorageValue::from(convert::from_js(value)),
    }
}

fn to_js(value: &StorageValue) -> JsValue {
    convert::to_js(&value.to_json())
}

impl StorageApi for Storage {
    fn get(
        &self,
        keys: &[&str],
        try_parse_json: bool,
        callback: Box<dyn FnOnce(bool, Vec<StorageValue>)>,
    ) {
        if !self.in_flight.begin("storage.get") {
            return;
        }
        // JSON is parsed on this side of the FFI, so the web and the editor
        // mock behave the same.
        let result = self.module.call2(
            "get",
            &convert::js_string_array(keys),
            &JsValue::from_bool(false),
        );
        settle(
            result,
            &self.in_flight,
            "storage.get",
            Box::new(move |ok, value| {
                let values = if ok {
                    convert::array_to_vec(&value)
                        .iter()
                        .map(|item| from_js(item, try_parse_json))
                        .collect()
                } else {
                    Vec::new()
                };
                callback(ok, values);
            }),
        );
    }

    fn set(&self, pairs: &[(&str, StorageValue)], callback: Box<dyn FnOnce(bool)>) {
        if !self.in_flight.begin("storage.set") {
            return;
        }
        let keys_of: Vec<&str> = pairs.iter().map(|(key, _)| *key).collect();
        let keys = convert::js_string_array(&keys_of);
        let values: Vec<JsValue> = pairs.iter().map(|(_, value)| to_js(value)).collect();
        let result = self.module.call2("set", &keys, &convert::js_array(&values));
        settle(
            result,
            &self.in_flight,
            "storage.set",
            Box::new(move |ok, _| callback(ok)),
        );
    }

    fn delete(&self, keys: &[&str], callback: Box<dyn FnOnce(bool)>) {
        if !self.in_flight.begin("storage.delete") {
            return;
        }
        let result = self.module.call1("delete", &convert::js_string_array(keys));
        settle(
            result,
            &self.in_flight,
            "storage.delete",
            Box::new(move |ok, _| callback(ok)),
        );
    }
}
