# playgama-bridge

Rust binding for the [Playgama Bridge](https://bridge.playgama.com) web SDK.

```rust
let bridge = playgama_bridge::load(Default::default()).await?;   // wasm only
bridge.platform.send_message(playgama_bridge::PlatformMessage::GameReady);
bridge.advertisement.interstitial_state_changed().connect(|state| { /* ... */ });
```

## Layout

| File | Godot counterpart |
| --- | --- |
| `src/types.rs` | the `Bridge.*` string constants, as enums, plus `StorageValue` |
| `src/api.rs` | one trait per module, the shape of `bridge.gd`'s accessors |
| `src/js/` | the live `window.bridge`, one file per `modules/*/*.gd` |
| `src/mock.rs` | the `modules/*/*_editor_mock.gd` stand-ins |
| `src/loader.rs` | the script block in `template/index.html` |
| `src/signal.rs` | GDScript `signal` |
| `src/inflight.rs` | the `if _callback != null: return` guards |

Fourteen modules, each with a browser implementation and an editor stand-in:
`platform`, `device`, `player`, `storage`, `advertisement`, `social`,
`leaderboards`, `payments`, `achievements`, `remote_config`, `cross_promo`,
`tasks`, `daily_rewards`, `notifications`.

## Behaviour carried over from the ref. (Godot) addon

- `Bridge::new()` picks the live SDK when the page has one and the editor
  stand-ins otherwise, the same branch `Bridge._ready` takes. A module the SDK
  left out falls back to its own stand-in, so one missing optional module costs
  only itself.
- On the live SDK there is one call in flight per operation: a second
  `storage.get` is dropped, not queued, and the slot is released before the
  callback runs, so a handler may start the same call again. The editor mocks
  answer immediately and so have nothing to guard.
- Every promise call hands its result to one callback whose first argument says
  whether the promise resolved or rejected, which is what the
  `then`/`catch` pair collapses to. A method that hands back a bare value
  instead of a promise counts as a rejection.
- State names are enums that round-trip their JS text, and an unknown string
  from a newer platform arrives as `None` instead of failing.
- `storage` keeps the SDK's text format: scalars are JSON text, and
  `StorageValue::deserialize` is `utils.deserialize_value` including its
  `"007"`, `"1.0"` and `"NaN"` round-trip rules.
- `loader` tries `https://bridge.playgama.com/v2/stable/playgama-bridge.js`
  first and falls back to `./playgama-bridge.js` after two seconds or on an
  error, tearing the remote tag down and clearing `window.bridge` first so a
  late arrival cannot overwrite the working bridge.

## Differences worth knowing

- `bridge.engine` is set to `"html5"`. The field is a free-form engine id —
  the Godot template sends `"godot-4"`, the Cocos guide `"cocos"` — and Bridge
  only copies it into its config, so nothing validates it. Override it with
  `LoadOptions::engine`.
- The editor mocks keep cloud storage in memory instead of writing
  `user://<key>.save`, and use an in-memory store rather than files.
- `is_advanced_banners_supported` and `advanced_banners_state` answer `false`
  and `None` on the mock. The Godot editor mock leaves them undefined, which
  would throw if the game asked.
- Signals are append-only, like Godot's; there is no disconnect. Connect once
  during setup.

## Checked against the live SDK

Verified against `playgama-bridge.js` v2 stable and the Bridge wiki: all 14
module names, 49 method names, 6 event names, the state and message strings, and
the `on`/`off`/`once` subscription API are present and named as this crate calls
them.

- `storage.get(keys, parseJson = true)`. This crate always passes `false` and
  parses Rust-side, matching `storage.gd`, so the web and the editor mock agree.
  A `null` value in `storage.set` **deletes** the key, which
  `StorageValue::Null` gives you.
- `storage.set(keys, values)` rejects when the arrays differ in length; a
  single-pair write cannot trip it.
- `sendMessage("game_ready")` rejects if it is sent twice, and removes the
  SDK's `#loading-overlay`. Send it exactly once.
- Every method returns a promise; one that hands back a bare value counts as a
  rejection here.
- `platform.isAudioEnabled` and the ad states must be **read** at start as well
  as subscribed to — the events only report changes. The caller's job, not the
  binding's.

Docs: <https://wiki.playgama.com/playgama/bridge-sdk/api> ·
<https://github.com/playgama/bridge> (LGPL-3.0 core SDK)

## Licence

MPL-2.0.