# playgama-bridge

Rust binding for the [Playgama Bridge](https://bridge.playgama.com) web SDK.

```rust
let bridge = playgama_bridge::load(Default::default()).await?;   // wasm only
bridge.root.set_game_version("1.0.0"); // optional( dev-tracking)
bridge.platform.send_message(playgama_bridge::PlatformMessage::GameReady);
bridge.advertisement.interstitial_state_changed().connect(|state| { /* ... */ });
```

A promise call reports the platform's own reason rather than a bare `false`:

```rust
bridge.notifications.schedule(
    Some(&serde_json::json!({ "id": "remind", "title": "…", "description": "…" })),
    Box::new(|result| match result {
        Ok(()) => { /* scheduled */ }
        Err(error) => match error.code() {
            Some(playgama_bridge::ErrorCode::NotificationInvalidParameters) => { /* … */ }
            _ => println!("{error}"),
        },
    }),
);
```

`examples/game_shell.rs` is a whole game's setup: read at start, signal per
change, one error code per call. `cargo run --example game_shell` runs it
against the desktop stand-ins.

## Layout

| File | Godot counterpart |
| --- | --- |
| `src/types.rs` | the `Bridge.*` string constants, as enums, plus `StorageValue`, `BridgeError`, `ScreenSize`, `SafeArea` |
| `src/api.rs` | one trait per module, the shape of `bridge.gd`'s accessors |
| `src/js/` | the live `window.bridge`, one file per `modules/*/*.gd` |
| `src/mock.rs` | the `modules/*/*_editor_mock.gd` stand-ins |
| `src/loader.rs` | the script block in `template/index.html` |
| `src/signal.rs` | GDScript `signal` |
| `src/inflight.rs` | the `if _callback != null: return` guards |

Sixteen modules, each with a browser implementation and an editor stand-in:
`platform`, `device`, `player`, `storage`, `advertisement`, `social`,
`leaderboards`, `payments`, `achievements`, `remote_config`, `clipboard`,
`analytics`, `notifications`, `daily_rewards`, `tasks`, `cross_promo`, plus
`bridge.root` for `version`, `isInitialized`, `options` and `gameVersion`.

## Behaviour carried over from the ref. (Godot) addon

- `Bridge::new()` picks the live SDK when the page has one and the editor
  stand-ins otherwise, the same branch `Bridge._ready` takes. A module the SDK
  left out falls back to its own stand-in, so one missing optional module costs
  only itself.
- On the live SDK there is one call in flight per operation: a second
  `storage.get` is dropped, not queued, and the slot is released before the
  callback runs, so a handler may start the same call again. The editor mocks
  answer immediately and so have nothing to guard.
- Every promise call hands its result to one callback: `Ok` when the promise
  resolved, `Err` carrying the `BridgeError` it rejected with. A method that
  hands back a bare value instead of a promise counts as a rejection.
- State names are enums that round-trip their JS text, and an unknown string
  from a newer platform arrives as `None` instead of failing.
- `storage` keeps the SDK's text format: a string is stored as it is, anything
  else becomes JSON text, and reading parses with `JSON.parse`, so
  `StorageValue` round-trips what the platform would return.
- `loader` tries `https://bridge.playgama.com/v2/stable/playgama-bridge.js`
  first and falls back to `./playgama-bridge.js` after two seconds or on an
  error, tearing the remote tag down and clearing `window.bridge` first so the
  fallback can install itself.

## Differences worth knowing

- `bridge.engine` is set to `"html5"`. The field is a free-form engine id; the
  Godot template sends `"godot-4"` and the Cocos guide `"cocos"`, and Bridge
  only copies it into its config, so nothing validates it. Override it with
  `LoadOptions::engine`.
- The editor mocks keep cloud storage in memory instead of writing
  `user://<key>.save`, and use an in-memory store rather than files.
- `is_advanced_banners_supported` and `advanced_banners_state` answer `false`
  and `None` on the mock. The Godot editor mock leaves them undefined, which
  would throw if the game asked.
- Signals are append-only, like Godot's; there is no disconnect. Connect once
  during setup.
- The editor mocks answer `Err` with no code for an unsupported feature, and
  the notifications mock names `NOTIFICATIONS_NOT_SUPPORTED`, which is what the
  SDK throws there.
- `device.orientation`, `screen_size_changed` and `safe_area` are plain-JS-only
  in the SDK; a wasm build can use them, and a desktop build sees the mock's
  `Landscape` and zero insets.
- `analytics.send` is fire-and-forget. The SDK batches events itself and drops
  them on `file://`, on localhost, and on any platform that blocks external
  calls. Nothing reports that back.
- The loader cannot order the CDN and the local fallback. The SDK installs
  itself only when both of its globals are free, so a CDN fetch that is still
  in flight when the tag is torn down can land in the gap and take the page
  instead. The Godot template has the same race.

## Checked against the live SDK

Verified against `playgama-bridge.js` **v2.3.0** and the Bridge wiki: all 16
module names, the method names, the 15 game-visible event names, the state and
message strings, the platform-id and error-code tables, and the `on`, `off` and
`once` subscription API are present and named as this crate calls them.

- `storage.get(keys, parseJson = true)` resolves to a bare value, not an array,
  when given a bare key. This crate always passes an array and parses Rust-side,
  so one entry per key comes back in the order asked for and the web and the
  editor mock agree. A `null` or empty value in `storage.set` **deletes** the
  key, which `StorageValue::Null` gives you.
- `storage.set(keys, values)` rejects when the arrays differ in length; a
  single-pair write cannot trip it.
- `sendMessage("game_ready")` rejects if it is sent twice, and removes the
  SDK's `#loading-overlay`. Send it exactly once.
- Every method returns a promise; one that hands back a bare value counts as a
  rejection here.
- `platform.isAudioEnabled`, `platform.isPaused` and the ad states must be
  **read** at start as well as subscribed to. The events only report changes.
- The SDK runs **two** event buses. `visibility_state_changed` and
  `platform_storage_availability_changed` are emitted on the platform's internal
  bus and never forwarded to the game's, so no module here exposes them. The
  other fifteen do reach a game, and every signal here is subscribed on
  `window.bridge` itself. `on`/`off`/`once` exist on only four prototypes
  (`bridge`, `platform`, `device`, `advertisement`), and all four share one bus,
  so a per-module subscription would have covered 9 of the 15.
- Storage text is `JSON.stringify` on the way in and `JSON.parse` on the way
  out, which means `1.0` is stored as `"1"` and comes back an int, `1e3` comes
  back as `1000`, and `"007"` and `"NaN"` stay the text they were because no
  `JSON.stringify` produces either. `StorageValue::stored_text` is what says a
  `null` or an empty string is the delete the SDK treats it as.
- On the live SDK a second call for an operation already in flight is dropped
  **and its callback is never invoked**, which is the Godot addon's behaviour.
  Read two keys by waiting for the first callback. Off the web every mock
  answers immediately, so nothing is dropped there.
- A handler may emit the signal it is connected to. The SDK does that itself
  when `hideBanner` fires `banner_state_changed`. The nested value is queued and
  delivered once the current pass finishes rather than inside it.
- Rejections carry a `BridgeError` with a `code` when the SDK has one, and
  reject bare when a feature is simply unsupported. `error.code()` being `None`
  is the second case, not a parse failure.
- `getCurrentReward` resolves with the whole reward entry from the config,
  which is `{ id, type, amount }` and not an id string, so it is an optional
  `ObjectMap`.
- `getAddToFavoritesReward` and `getAddToHomeScreenReward` have no documented
  shape, so they are typed as an optional `ObjectMap` too.
- A rejected `notifications.schedule` carries the field that was wrong in
  `BridgeError::detail` (`originalError`), not only the generic text.

Docs: <https://wiki.playgama.com/playgama/bridge-sdk/api> ·
<https://github.com/playgama/bridge> (LGPL-3.0 core SDK)

## Licence

MPL-2.0.
