//! Conversions between `serde_json::Value` and JavaScript values, plus the
//! small readers the modules use to walk into the bridge's objects.

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::{JsCast as _, JsValue};

/// `utils.convert_to_js`: objects and arrays become real JS ones, scalars pass
/// through.
pub fn to_js(value: &serde_json::Value) -> JsValue {
    match value {
        serde_json::Value::Null => JsValue::NULL,
        serde_json::Value::Bool(v) => JsValue::from_bool(*v),
        // JS numbers are all doubles, which is exactly what the SDK sends back.
        serde_json::Value::Number(n) => JsValue::from_f64(n.as_f64().unwrap_or_default()),
        serde_json::Value::String(v) => JsValue::from_str(v),
        serde_json::Value::Array(items) => {
            let array = Array::new();
            for item in items {
                array.push(&to_js(item));
            }
            array.into()
        }
        serde_json::Value::Object(entries) => {
            let object = Object::new();
            for (key, item) in entries {
                let _ = Reflect::set(&object, &JsValue::from_str(key), &to_js(item));
            }
            object.into()
        }
    }
}

/// Recursive `utils.convert_to_gd_object` / `_js_to_gd`: JS values become
/// `serde_json` ones. `undefined` and functions are indistinguishable from
/// `null` across the FFI, so both land there.
pub fn from_js(value: &JsValue) -> serde_json::Value {
    if value.is_null() || value.is_undefined() {
        return serde_json::Value::Null;
    }
    if Array::is_array(value) {
        let array = Array::from(value);
        let mut items = Vec::with_capacity(array.length() as usize);
        for index in 0..array.length() {
            items.push(from_js(&array.get(index)));
        }
        return serde_json::Value::Array(items);
    }
    if typeof_object(value) {
        let object: &Object = value.unchecked_ref();
        let mut map = serde_json::Map::new();
        for key in Object::keys(object) {
            let Some(key) = key.as_string() else { continue };
            let item = Reflect::get(object, &JsValue::from_str(&key)).unwrap_or(JsValue::UNDEFINED);
            map.insert(key, from_js(&item));
        }
        return serde_json::Value::Object(map);
    }
    if let Some(text) = value.as_string() {
        return serde_json::Value::String(text);
    }
    if let Some(flag) = value.as_bool() {
        return serde_json::Value::Bool(flag);
    }
    if let Some(number) = value.as_f64() {
        return serde_json::Number::from_f64(number)
            .map_or(serde_json::Value::Null, serde_json::Value::Number);
    }
    serde_json::Value::Null
}

pub fn object_to_map(value: &JsValue) -> crate::api::ObjectMap {
    match from_js(value) {
        serde_json::Value::Object(map) => map,
        _ => crate::api::ObjectMap::new(),
    }
}

pub fn array_to_vec_of_maps(value: &JsValue) -> Vec<crate::api::ObjectMap> {
    array_to_vec(value).iter().map(object_to_map).collect()
}

pub fn array_to_vec(value: &JsValue) -> Vec<JsValue> {
    if !Array::is_array(value) {
        return Vec::new();
    }
    let array = Array::from(value);
    (0..array.length()).map(|index| array.get(index)).collect()
}

pub fn typeof_object(value: &JsValue) -> bool {
    value.js_typeof() == JsValue::from_str("object")
}

pub fn as_string(value: &JsValue) -> Option<String> {
    value.as_string()
}

/// A JS array of values.
pub fn js_array<T: AsRef<JsValue>>(items: &[T]) -> JsValue {
    let array = Array::new();
    for item in items {
        array.push(item.as_ref());
    }
    array.into()
}

/// A JS array of strings, which is how the storage module takes its keys.
pub fn js_string_array(items: &[&str]) -> JsValue {
    let array = Array::new();
    for item in items {
        array.push(&JsValue::from_str(item));
    }
    array.into()
}

/// An optional argument, which the SDK sees as `null` when there is none.
pub fn arg(value: Option<&serde_json::Value>) -> JsValue {
    value.map_or(JsValue::NULL, to_js)
}
