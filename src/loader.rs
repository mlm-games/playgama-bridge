//! Getting `window.bridge` onto the page, in Rust instead of the script block
//! the Godot template carries.
//!
//! The CDN script is tried first, exactly as `template/index.html` does: on an
//! error, or if it has not arrived within `timeout_ms`, a local
//! `playgama-bridge.js` is loaded instead.

use std::cell::RefCell;
use std::rc::Rc;

use js_sys::{Array, Function, Promise, Reflect};
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::*;
use wasm_bindgen_futures::JsFuture;

use crate::Bridge;
use crate::js;

pub const REMOTE_BRIDGE_URL: &str = "https://bridge.playgama.com/v2/stable/playgama-bridge.js";
pub const LOCAL_BRIDGE_URL: &str = "./playgama-bridge.js";

/// What `bridge.engine` is set to before `initialize`. The Godot template sends
/// `"godot-4"`; a wasm build is plain HTML5 to the platform.
pub const DEFAULT_ENGINE: &str = "html5";

/// Reports loading progress on the 0–100 scale the Godot shell uses.
pub fn set_game_loading_progress(progress: f64) {
    let Some(root) = js::global_bridge() else {
        return;
    };
    let Ok(set) = Reflect::get(&root, &JsValue::from_str("setGameLoadingProgress"))
        .and_then(|value| value.dyn_into::<Function>())
    else {
        return;
    };
    let _ = set.call1(&root, &JsValue::from_f64(progress));
}

#[derive(Clone, Debug)]
pub struct LoadOptions {
    pub remote_url: &'static str,
    pub local_url: &'static str,
    /// How long the remote script gets before the local one is tried.
    pub timeout_ms: i32,
    /// `bridge.engine`; `None` leaves whatever the page set.
    pub engine: Option<&'static str>,
}

impl Default for LoadOptions {
    fn default() -> Self {
        Self {
            remote_url: REMOTE_BRIDGE_URL,
            local_url: LOCAL_BRIDGE_URL,
            timeout_ms: 2000,
            engine: Some(DEFAULT_ENGINE),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoadError {
    /// No `document`, so no script tag could be added.
    NoDocument,
    /// Neither the CDN nor the local script produced a bridge.
    NoBridge,
    /// The bridge loaded but is missing one of the fourteen modules.
    IncompleteBridge,
    /// `bridge.initialize()` rejected.
    InitializeFailed,
}

impl std::fmt::Display for LoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            Self::NoDocument => "the page has no document to load the bridge into",
            Self::NoBridge => "playgama-bridge.js did not define window.bridge",
            Self::IncompleteBridge => "window.bridge is missing one of the fourteen modules",
            Self::InitializeFailed => "bridge.initialize() rejected",
        };
        f.write_str(text)
    }
}

impl std::error::Error for LoadError {}

/// Load the SDK if the page has not already, then initialize it.
pub async fn load(options: LoadOptions) -> Result<Bridge, LoadError> {
    if js::global_bridge().is_none() {
        let remote = inject(options.remote_url)?;
        let settled = when_loaded(&remote, options.timeout_ms).await;
        if !settled || js::global_bridge().is_none() {
            // The template tears the remote tag down and clears the global
            // before the local one, so a late CDN arrival cannot win.
            remote.remove();
            js::clear_global_bridge();
            let local = inject(options.local_url)?;
            if !when_loaded(&local, options.timeout_ms).await || js::global_bridge().is_none() {
                return Err(LoadError::NoBridge);
            }
        }
    }
    initialize(options.engine).await
}

async fn initialize(engine: Option<&'static str>) -> Result<Bridge, LoadError> {
    let root = js::global_bridge().ok_or(LoadError::NoBridge)?;
    if let Some(engine) = engine {
        let _ = Reflect::set(
            &root,
            &JsValue::from_str("engine"),
            &JsValue::from_str(engine),
        );
    }
    let initialize = Reflect::get(&root, &JsValue::from_str("initialize"))
        .ok()
        .and_then(|value| value.dyn_into::<Function>().ok())
        .ok_or(LoadError::IncompleteBridge)?;
    let promise = initialize
        .call0(&root)
        .map_err(|_| LoadError::InitializeFailed)?;
    JsFuture::from(Promise::from(promise))
        .await
        .map_err(|_| LoadError::InitializeFailed)?;
    let bridge = Bridge::new();
    if !bridge.is_web() {
        return Err(LoadError::IncompleteBridge);
    }
    Ok(bridge)
}

fn inject(url: &str) -> Result<web_sys::HtmlScriptElement, LoadError> {
    let document = web_sys::window()
        .and_then(|window| window.document())
        .ok_or(LoadError::NoDocument)?;
    let script = document
        .create_element("script")
        .map_err(|_| LoadError::NoDocument)?
        .dyn_into::<web_sys::HtmlScriptElement>()
        .map_err(|_| LoadError::NoDocument)?;
    script.set_src(url);
    let head = document.head().ok_or(LoadError::NoDocument)?;
    head.append_child(&script)
        .map_err(|_| LoadError::NoDocument)?;
    Ok(script)
}

/// `true` when the tag fires `load`, `false` when it fires `error` or
/// `timeout_ms` passes first.
///
/// The promise fulfils with a boolean so a timeout (which fulfils with
/// `undefined`) can be told from an `error` event.
async fn when_loaded(script: &web_sys::HtmlScriptElement, timeout_ms: i32) -> bool {
    let settle: Rc<RefCell<Option<Function>>> = Rc::new(RefCell::new(None));
    let promise = resolver(Rc::clone(&settle));

    let listen = |loaded: bool| {
        let settle = Rc::clone(&settle);
        Closure::wrap(Box::new(move |_event: web_sys::Event| {
            if let Some(finish) = settle.borrow_mut().take() {
                let _ = finish.call1(&JsValue::NULL, &JsValue::from_bool(loaded));
            }
        }) as Box<dyn FnMut(web_sys::Event)>)
    };
    let on_load = listen(true);
    let on_error = listen(false);

    let add = |event: &str, listener: &Closure<dyn FnMut(web_sys::Event)>| {
        script.add_event_listener_with_callback(event, listener.as_ref().unchecked_ref())
    };
    let listening = add("load", &on_load).is_ok() && add("error", &on_error).is_ok();
    if !listening {
        // The DOM must not be left holding a callback whose wasm side is gone.
        remove(script, "load", &on_load);
        remove(script, "error", &on_error);
        return false;
    }
    let loaded = JsFuture::from(race_with_timeout(Promise::from(promise), timeout_ms))
        .await
        .is_ok_and(|value| value.is_truthy());
    remove(script, "load", &on_load);
    remove(script, "error", &on_error);
    loaded
}

fn remove(
    script: &web_sys::HtmlScriptElement,
    event: &str,
    listener: &Closure<dyn FnMut(web_sys::Event)>,
) {
    let _ = script.remove_event_listener_with_callback(event, listener.as_ref().unchecked_ref());
}

/// A promise whose resolver waits in `slot` until a listener needs it.
fn resolver(slot: Rc<RefCell<Option<Function>>>) -> JsValue {
    Promise::new(&mut |resolve, _reject| {
        *slot.borrow_mut() = Some(resolve);
    })
    .into()
}

/// Resolves `undefined` when the timer wins, which is how `when_loaded` tells a
/// timeout from an `error` event.
fn race_with_timeout(promise: Promise, timeout_ms: i32) -> Promise {
    let timer = Promise::new(&mut |resolve, _reject| {
        let settled = match web_sys::window() {
            Some(window) => window
                .set_timeout_with_callback_and_timeout_and_arguments_0(&resolve, timeout_ms)
                .is_ok(),
            None => false,
        };
        // Without a timer the promise would never settle, and the game would
        // wait for it forever.
        if !settled {
            let _ = resolve.call0(&JsValue::NULL);
        }
    });
    let race = Array::of2(&JsValue::from(promise), &timer.into());
    Promise::race(race.as_ref())
}
