//! One module object of `window.bridge`, plus the promise and event plumbing
//! every module shares.

mod achievements;
mod advertisement;
mod cross_promo;
mod daily_rewards;
mod device;
mod leaderboards;
mod notifications;
mod payments;
mod platform;
mod player;
mod remote_config;
mod social;
mod storage;
mod tasks;

use std::cell::RefCell;
use std::rc::Rc;
use std::str::FromStr;

use js_sys::{Array, Function, Promise, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::{JsFuture, spawn_local};

use crate::api::*;
use crate::convert;
use crate::inflight::InFlight;
use crate::signal::Signal;

pub(crate) use achievements::Achievements;
pub(crate) use advertisement::Advertisement;
pub(crate) use cross_promo::CrossPromo;
pub(crate) use daily_rewards::DailyRewards;
pub(crate) use device::Device;
pub(crate) use leaderboards::Leaderboards;
pub(crate) use notifications::Notifications;
pub(crate) use payments::Payments;
pub(crate) use platform::Platform;
pub(crate) use player::Player;
pub(crate) use remote_config::RemoteConfig;
pub(crate) use social::Social;
pub(crate) use storage::Storage;
pub(crate) use tasks::Tasks;

/// `window.bridge`, when the page has loaded the SDK.
pub(crate) fn global_bridge() -> Option<JsValue> {
    let bridge = Reflect::get(&js_sys::global(), &JsValue::from_str("bridge")).ok()?;
    (!bridge.is_undefined() && !bridge.is_null()).then_some(bridge)
}

/// Drops a bridge the page loaded in place of the one we are about to inject,
/// the same reset `template/index.html` does before its local fallback.
pub(crate) fn clear_global_bridge() {
    for key in ["bridge", "playgamaBridge"] {
        let _ = Reflect::set(&js_sys::global(), &JsValue::from_str(key), &JsValue::NULL);
    }
}

fn module_of(root: &JsValue, name: &str) -> Option<Module> {
    let object = Reflect::get(root, &JsValue::from_str(name)).ok()?;
    (!object.is_undefined() && !object.is_null()).then_some(Module { object })
}

/// The root object every module hangs off. A module the SDK left out is simply
/// absent: one missing optional module must not cost the game the other
/// thirteen.
pub(crate) fn modules() -> Option<Modules> {
    global_bridge().map(|root| Modules { root })
}

pub(crate) struct Modules {
    root: JsValue,
}

impl Modules {
    pub(crate) fn get(&self, name: &'static str) -> Option<Module> {
        module_of(&self.root, name)
    }
}

/// A handle on one module object. Every accessor is total: a platform that
/// leaves a property off reports "unknown", never a panic.
#[derive(Clone)]
pub(crate) struct Module {
    object: JsValue,
}

impl Module {
    fn get(&self, key: &str) -> Option<JsValue> {
        let value = Reflect::get(&self.object, &JsValue::from_str(key)).ok()?;
        (!value.is_undefined()).then_some(value)
    }

    fn method(&self, key: &str) -> Option<Function> {
        self.get(key)?.dyn_into::<Function>().ok()
    }

    fn string(&self, key: &str) -> Option<String> {
        self.get(key)?.as_string()
    }

    fn bool(&self, key: &str) -> bool {
        self.get(key).and_then(|v| v.as_bool()).unwrap_or(false)
    }

    fn f64(&self, key: &str) -> f64 {
        self.get(key).and_then(|v| v.as_f64()).unwrap_or_default()
    }

    fn text<T: FromStr>(&self, key: &str) -> Option<T> {
        T::from_str(&self.string(key)?).ok()
    }

    fn map(&self, key: &str) -> ObjectMap {
        self.get(key)
            .map_or_else(ObjectMap::new, |v| convert::object_to_map(&v))
    }

    fn call0(&self, key: &str) -> JsValue {
        self.method(key).map_or(JsValue::UNDEFINED, |m| {
            m.call0(&self.object).unwrap_or(JsValue::UNDEFINED)
        })
    }

    fn call1(&self, key: &str, first: &JsValue) -> JsValue {
        self.method(key).map_or(JsValue::UNDEFINED, |m| {
            m.call1(&self.object, first).unwrap_or(JsValue::UNDEFINED)
        })
    }

    fn call2(&self, key: &str, first: &JsValue, second: &JsValue) -> JsValue {
        self.method(key).map_or(JsValue::UNDEFINED, |m| {
            m.call2(&self.object, first, second)
                .unwrap_or(JsValue::UNDEFINED)
        })
    }

    /// A method called with an optional argument, which JS sees as `null` when
    /// there is none, the way the GDScript passes a `null` through.
    fn call_opt(&self, key: &str, arg: Option<JsValue>) -> JsValue {
        self.call1(key, &arg.unwrap_or(JsValue::NULL))
    }
}

/// Keeps the JS callbacks we handed to `on` alive for as long as the module is.
#[derive(Default)]
pub(crate) struct Listeners(RefCell<Vec<Closure<dyn FnMut(&JsValue)>>>);

impl Listeners {
    /// `module.on(event, handler)`. The platform calls `handler(value)`
    /// with the new value as the first argument; some events hand the
    /// handler an array instead, so both shapes are accepted.
    pub(crate) fn listen<T, F>(&self, module: &Module, event: &str, parse: F, signal: &Signal<T>)
    where
        T: Clone + 'static,
        F: Fn(&JsValue) -> T + 'static,
    {
        let Some(on) = module.method("on") else {
            return;
        };
        let sink = signal.clone();
        let closure = Closure::wrap(Box::new(move |args: &JsValue| {
            let first = if Array::is_array(args) {
                convert::array_to_vec(args)
                    .first()
                    .cloned()
                    .unwrap_or(JsValue::UNDEFINED)
            } else {
                args.clone()
            };
            sink.emit(parse(&first));
        }) as Box<dyn FnMut(&JsValue)>);
        let handler = closure.as_ref().clone();
        if on
            .call2(&module.object, &JsValue::from_str(event), handler.as_ref())
            .is_ok()
        {
            self.0.borrow_mut().push(closure);
        }
    }
}

/// `promise.then(cb).catch(cb)`, collapsed into one callback whose flag says
/// which side ran. Releases `slot` first, as `_on_js_*_then` does, so the
/// handler is free to start the same call again.
pub(crate) fn settle(
    result: JsValue,
    in_flight: &InFlight,
    slot: &'static str,
    callback: Box<dyn FnOnce(bool, JsValue)>,
) {
    let guard = in_flight.clone();
    let finish = move |ok: bool, value: JsValue, callback: Box<dyn FnOnce(bool, JsValue)>| {
        guard.end(slot);
        callback(ok, value);
    };
    let Ok(promise) = result.dyn_into::<Promise>() else {
        finish(false, JsValue::UNDEFINED, callback);
        return;
    };
    spawn_local(async move {
        match JsFuture::from(promise).await {
            Ok(value) => finish(true, value, callback),
            Err(_) => finish(false, JsValue::UNDEFINED, callback),
        }
    });
}

/// The trait objects `Bridge` hands out, built from the live SDK.
pub(crate) struct Web {
    pub(crate) platform: Rc<dyn PlatformApi>,
    pub(crate) device: Rc<dyn DeviceApi>,
    pub(crate) player: Rc<dyn PlayerApi>,
    pub(crate) storage: Rc<dyn StorageApi>,
    pub(crate) advertisement: Rc<dyn AdvertisementApi>,
    pub(crate) social: Rc<dyn SocialApi>,
    pub(crate) leaderboards: Rc<dyn LeaderboardsApi>,
    pub(crate) payments: Rc<dyn PaymentsApi>,
    pub(crate) achievements: Rc<dyn AchievementsApi>,
    pub(crate) remote_config: Rc<dyn RemoteConfigApi>,
    pub(crate) cross_promo: Rc<dyn CrossPromoApi>,
    pub(crate) tasks: Rc<dyn TasksApi>,
    pub(crate) daily_rewards: Rc<dyn DailyRewardsApi>,
    pub(crate) notifications: Rc<dyn NotificationsApi>,
}

/// `None` when the page has no bridge at all, so the caller can fall back to
/// the editor stand-ins. A module the SDK left out falls back to its own
/// stand-in, so a missing optional module costs only itself.
pub(crate) fn build(in_flight: &InFlight) -> Option<Web> {
    let modules = modules()?;
    Some(Web {
        platform: Platform::attach(&modules, in_flight).unwrap_or_else(crate::mock::platform),
        device: Device::attach(&modules).unwrap_or_else(crate::mock::device),
        player: Player::attach(&modules, in_flight).unwrap_or_else(crate::mock::player),
        storage: Storage::attach(&modules, in_flight).unwrap_or_else(crate::mock::storage),
        advertisement: Advertisement::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::advertisement),
        social: Social::attach(&modules, in_flight).unwrap_or_else(crate::mock::social),
        leaderboards: Leaderboards::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::leaderboards),
        payments: Payments::attach(&modules, in_flight).unwrap_or_else(crate::mock::payments),
        achievements: Achievements::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::achievements),
        remote_config: RemoteConfig::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::remote_config),
        cross_promo: CrossPromo::attach(&modules).unwrap_or_else(crate::mock::cross_promo),
        tasks: Tasks::attach(&modules, in_flight).unwrap_or_else(crate::mock::tasks),
        daily_rewards: DailyRewards::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::daily_rewards),
        notifications: Notifications::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::notifications),
    })
}
