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

/// An optional object payload: a resolved call whose value is not an object
/// carries nothing rather than an empty map. `typeof null` is `"object"` in JS,
/// so null is checked first. Otherwise a declined purchase reading `null` would
/// come back as an empty map and a caller could not tell them apart.
pub fn optional_object(value: &JsValue) -> Option<crate::api::ObjectMap> {
    (!value.is_null() && typeof_object(value) && !Array::is_array(value))
        .then(|| object_to_map(value))
}

/// The reason a promise rejected. The SDK throws `BridgeError` with a `code`
/// when it has one and rejects bare when a feature is simply unsupported, so a
/// missing code is a real answer and not a parse failure.
pub fn error_from_js(value: &JsValue) -> crate::types::BridgeError {
    // A bare `Promise.reject()` has no reason, which is what an unsupported
    // feature looks like.
    if !typeof_object(value) {
        return match value.as_string() {
            Some(text) => crate::types::BridgeError::detailed(None, None, Some(text), None),
            None => crate::types::BridgeError::rejected(),
        };
    }
    let field = |name: &str| {
        Reflect::get(value, &JsValue::from_str(name))
            .ok()
            .filter(|item| !item.is_null() && !item.is_undefined())
            .map(|item| match item.as_string() {
                Some(text) => text,
                // `originalError` is a plain string on every throw the SDK
                // makes; anything else is rendered rather than dropped.
                None => from_js(&item).to_string(),
            })
    };
    // A code this crate does not know is kept as text rather than dropped, so
    // a platform newer than the binding still says what failed.
    let raw_code = field("code");
    let code = raw_code.as_deref().and_then(|text| text.parse().ok());
    crate::types::BridgeError::detailed(code, raw_code, field("message"), field("originalError"))
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
