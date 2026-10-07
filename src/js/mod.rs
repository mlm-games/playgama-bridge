//! One module object of `window.bridge`, plus the promise and event plumbing
//! every module shares.

mod achievements;
mod advertisement;
mod analytics;
mod clipboard;
mod cross_promo;
mod daily_rewards;
mod device;
mod leaderboards;
mod notifications;
mod payments;
mod platform;
mod player;
mod remote_config;
mod root;
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
use crate::types::BridgeResult;

pub(crate) use achievements::Achievements;
pub(crate) use advertisement::Advertisement;
pub(crate) use analytics::Analytics;
pub(crate) use clipboard::Clipboard;
pub(crate) use cross_promo::CrossPromo;
pub(crate) use daily_rewards::DailyRewards;
pub(crate) use device::Device;
pub(crate) use leaderboards::Leaderboards;
pub(crate) use notifications::Notifications;
pub(crate) use payments::Payments;
pub(crate) use platform::Platform;
pub(crate) use player::Player;
pub(crate) use remote_config::RemoteConfig;
pub(crate) use root::Root;
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
/// fifteen.
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

    /// Reads a `{ top, right, bottom, left }` insets object, each side
    /// optional.
    fn safe_area(&self, key: &str) -> crate::types::SafeArea {
        let Some(value) = self.get(key) else {
            return crate::types::SafeArea::default();
        };
        let side = |name: &str| {
            Reflect::get(&value, &JsValue::from_str(name))
                .ok()
                .and_then(|side| side.as_f64())
                .unwrap_or_default()
        };
        crate::types::SafeArea {
            top: side("top"),
            right: side("right"),
            bottom: side("bottom"),
            left: side("left"),
        }
    }

    /// The value a method handed back, or a marker carrying why there is none.
    /// Every SDK method this crate calls is present on a live bridge, so the
    /// marker only appears for an SDK older than this crate or for a method
    /// that threw. [`settle`] hands the reason to the caller's callback rather
    /// than dropping it.
    fn call0(&self, key: &str) -> JsValue {
        let Some(method) = self.method(key) else {
            return call_failed(&JsValue::from_str("the module has no such method"));
        };
        method
            .call0(&self.object)
            .unwrap_or_else(|thrown| call_failed(&thrown))
    }

    fn call1(&self, key: &str, first: &JsValue) -> JsValue {
        let Some(method) = self.method(key) else {
            return call_failed(&JsValue::from_str("the module has no such method"));
        };
        method
            .call1(&self.object, first)
            .unwrap_or_else(|thrown| call_failed(&thrown))
    }

    fn call2(&self, key: &str, first: &JsValue, second: &JsValue) -> JsValue {
        let Some(method) = self.method(key) else {
            return call_failed(&JsValue::from_str("the module has no such method"));
        };
        method
            .call2(&self.object, first, second)
            .unwrap_or_else(|thrown| call_failed(&thrown))
    }

    /// A method called with an optional argument, which JS sees as `null` when
    /// there is none, the way the GDScript passes a `null` through.
    fn call_opt(&self, key: &str, arg: Option<JsValue>) -> JsValue {
        self.call1(key, &arg.unwrap_or(JsValue::NULL))
    }
}

/// The marker key on a value that is a thrown error rather than a result.
const FAILED_KEY: &str = "__playgama_bridge_call_failed";

/// Wraps a thrown value so [`settle`] can tell "the call failed" from "the call
/// resolved with an object that happens to look like anything".
fn call_failed(thrown: &JsValue) -> JsValue {
    let marker = js_sys::Object::new();
    let _ = Reflect::set(&marker, &JsValue::from_str(FAILED_KEY), thrown);
    marker.into()
}

/// The value a call threw, when it threw.
fn thrown_by(result: &JsValue) -> Option<JsValue> {
    Reflect::get(result, &JsValue::from_str(FAILED_KEY))
        .ok()
        .filter(|thrown| !thrown.is_undefined() && !thrown.is_null())
}

/// Keeps the JS callbacks we handed to `on` alive for as long as the module is.
pub(crate) struct Listeners {
    root: Module,
    held: RefCell<Vec<Closure<dyn FnMut(&JsValue)>>>,
}

impl Default for Listeners {
    /// With no `window.bridge` there is nothing to subscribe to; every
    /// `listen` is then a no-op.
    fn default() -> Self {
        Self {
            root: Module {
                object: global_bridge().unwrap_or(JsValue::UNDEFINED),
            },
            held: RefCell::new(Vec::new()),
        }
    }
}

impl Listeners {
    /// `on(event, handler)` on the **root**, which is the only object in the
    /// SDK carrying it besides `platform`, `device` and `advertisement`. Every
    /// module shares one bus, so the root hears everything and the per-module
    /// listeners would only cover 9 of the 15 game-visible events.
    ///
    /// The platform calls `handler(value)` with the new value as the first
    /// argument; some events hand the handler an array instead, so both shapes
    /// are accepted.
    pub(crate) fn listen<T, F>(&self, event: &str, parse: F, signal: &Signal<T>)
    where
        T: Clone + 'static,
        F: Fn(&JsValue) -> T + 'static,
    {
        let Some(on) = self.root.method("on") else {
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
            .call2(
                &self.root.object,
                &JsValue::from_str(event),
                handler.as_ref(),
            )
            .is_ok()
        {
            // The JS side holds the handler now, so the wasm `Closure` has to
            // outlive this call or the next emit throws.
            self.held.borrow_mut().push(closure);
        }
    }
}

/// `promise.then(cb).catch(cb)`, collapsed into one callback that says which
/// side ran and hands the reason over on the failing one. Releases `slot`
/// first, as `_on_js_*_then` does, so the handler is free to start the same
/// call again.
pub(crate) fn settle(
    result: JsValue,
    in_flight: &InFlight,
    slot: &'static str,
    callback: Box<dyn FnOnce(BridgeResult<JsValue>)>,
) {
    let guard = in_flight.clone();
    // A method that handed back a bare value is a failure, not a resolution:
    // `dyn_into` consumes the value, so read the reason before giving up on it.
    // The reason is either what the method threw or the bare value itself, which
    // is how a missing method or a plain `undefined` reads as a refusal.
    let Ok(promise) = result.clone().dyn_into::<Promise>() else {
        guard.end(slot);
        let reason = thrown_by(&result).unwrap_or_else(|| result.clone());
        callback(Err(convert::error_from_js(&reason)));
        return;
    };
    spawn_local(async move {
        // The slot is released before the handler runs, so a handler may start
        // the same call again.
        let outcome = match JsFuture::from(promise).await {
            Ok(value) => Ok(value),
            Err(reason) => Err(convert::error_from_js(&reason)),
        };
        guard.end(slot);
        callback(outcome);
    });
}

/// The trait objects `Bridge` hands out, built from the live SDK.
pub(crate) struct Web {
    pub(crate) root: Rc<dyn RootApi>,
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
    pub(crate) clipboard: Rc<dyn ClipboardApi>,
    pub(crate) analytics: Rc<dyn AnalyticsApi>,
    pub(crate) notifications: Rc<dyn NotificationsApi>,
    pub(crate) daily_rewards: Rc<dyn DailyRewardsApi>,
    pub(crate) tasks: Rc<dyn TasksApi>,
    pub(crate) cross_promo: Rc<dyn CrossPromoApi>,
}

/// `None` when the page has no bridge at all, so the caller can fall back to
/// the editor stand-ins. A module the SDK left out falls back to its own
/// stand-in, so a missing optional module costs only itself.
pub(crate) fn build(in_flight: &InFlight) -> Option<Web> {
    let modules = modules()?;
    Some(Web {
        root: Root::attach(),
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
        clipboard: Clipboard::attach(&modules, in_flight).unwrap_or_else(crate::mock::clipboard),
        analytics: Analytics::attach(&modules).unwrap_or_else(crate::mock::analytics),
        notifications: Notifications::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::notifications),
        daily_rewards: DailyRewards::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::daily_rewards),
        tasks: Tasks::attach(&modules, in_flight).unwrap_or_else(crate::mock::tasks),
        cross_promo: CrossPromo::attach(&modules, in_flight)
            .unwrap_or_else(crate::mock::cross_promo),
    })
}
