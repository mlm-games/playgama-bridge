//! The shape of a real game's bridge setup: one call at launch, one signal per
//! thing worth reacting to, and every promise call reading its own error code.
//!
//! Build it for wasm to talk to a live platform; on the desktop it runs against
//! the editor stand-ins and does the same thing without a browser.
//!
//! ```text
//! cargo run --example game_shell
//! ```

use std::rc::Rc;

use playgama_bridge::{Bridge, DeviceOrientation, ErrorCode, PlatformMessage, RewardedState};

fn main() {
    let bridge = Bridge::new();
    println!(
        "playgama-bridge {:?} on {:?} via {:?}",
        bridge.version().unwrap_or_else(|| "unknown".into()),
        bridge.platform.platform_id().map(|id| id.as_str()),
        bridge.backend(),
    );

    // Read the state a signal will only report changes of.
    println!("audio enabled: {}", bridge.platform.is_audio_enabled());
    println!("paused: {}", bridge.platform.is_paused());
    println!(
        "orientation: {:?}, safe area {:?}",
        bridge
            .device
            .orientation()
            .unwrap_or(DeviceOrientation::Landscape),
        bridge.device.safe_area()
    );

    if let Some(language) = bridge.platform.language() {
        println!("language: {language}");
    }
    if bridge.platform.launch_source().is_some() {
        // A post reward is only owed once, on the launch that earned it.
        bridge
            .social
            .get_post_reward(Box::new(|outcome| match outcome {
                Ok(rewards) => {
                    for reward in rewards {
                        let id = reward.get("id").and_then(|id| id.as_str()).unwrap_or("?");
                        let amount = reward
                            .get("amount")
                            .and_then(|amount| amount.as_i64())
                            .unwrap_or_default();
                        println!("owed {amount} x {id}");
                    }
                }
                Err(error) => println!("post rewards unavailable: {error}"),
            }));
    }

    // Ads: subscribe before showing, and read the state at start as well.
    let rewarded = Rc::clone(&bridge.advertisement);
    bridge
        .advertisement
        .rewarded_state_changed()
        .connect(move |state| {
            if state == Some(RewardedState::Rewarded) {
                rewarded.check_adblock(Box::new(|outcome| {
                    println!("rewarded granted, adblock {:?}", outcome.unwrap_or(false));
                }));
            }
        });

    if bridge.advertisement.is_rewarded_supported() {
        // Warm the ad up while the player is on the menu, so the later show is
        // not waiting on a network round trip.
        bridge.advertisement.preload_rewarded(Some("revive"));
        bridge.advertisement.show_rewarded(Some("revive"));
    } else {
        bridge
            .advertisement
            .show_interstitial(Some("between_levels"));
    }

    bridge
        .advertisement
        .interstitial_state_changed()
        .connect(|state| println!("interstitial is now {state:?}"));

    // Persistence: the one call whose failure the player never sees, so a
    // refusal is logged rather than swallowed.
    let storage = Rc::clone(&bridge.storage);
    playgama_bridge::get_json::<serde_json::Value>(
        &*storage,
        "save",
        move |outcome| match outcome {
            Ok(Some(save)) => println!("resumed from {save}"),
            Ok(None) => println!("no save yet"),
            Err(error) if error.code() == Some(ErrorCode::StorageQuotaExceeded) => {
                println!("save is too big; starting fresh")
            }
            Err(error) => println!("could not read the save: {error}"),
        },
    );

    bridge
        .device
        .screen_size_changed()
        .connect(|size| println!("viewport is now {size:?}"));
    bridge
        .platform
        .pause_state_changed()
        .connect(|paused| println!("game loop {}", if paused { "held" } else { "running" }));
    bridge
        .platform
        .message_sent()
        .connect(|message| println!("told the platform: {message}"));

    // Notifications need a title and a description to be valid; the SDK
    // rejects anything else with a code that says which.
    if bridge.notifications.is_supported() {
        bridge.notifications.schedule(
            Some(&serde_json::json!({
                "id": "remind",
                "title": "Your run is waiting",
                "description": "Come back and pick it up",
                "delaySeconds": 3600,
            })),
            Box::new(|outcome| match outcome {
                Ok(()) => println!("reminder scheduled"),
                Err(error) if error.code() == Some(ErrorCode::NotificationInvalidParameters) => {
                    println!("reminder rejected: {}", error.message())
                }
                Err(error) => println!("reminder not scheduled: {error}"),
            }),
        );
    }

    // Clipboard, for the share-invite flow.
    bridge.clipboard.write(
        "https://my.game/invite/abc123",
        Box::new(|outcome| println!("invite copied: {}", outcome.is_ok())),
    );

    bridge.analytics.send(
        "session_start",
        Some(&serde_json::json!({ "version": "1.0.0" })),
    );

    // The platform learns the game is up last, so its own timer counts the
    // whole load rather than just this frame. Send it exactly once.
    bridge.platform.send_message(PlatformMessage::GameReady);
}
