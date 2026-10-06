//! The single-flight guard every module in `bridge.gd` puts in front of its
//! promise calls: while one `storage.get` is outstanding a second one is
//! dropped, not queued.

use std::cell::RefCell;
use std::rc::Rc;

/// Which operations of one `Bridge` are still waiting for the platform.
///
/// Shared by every module so a key like `"storage.get"` is unambiguous, which
/// is what the per-module GDScript guards amount to.
#[derive(Clone, Default)]
pub struct InFlight(Rc<RefCell<Vec<&'static str>>>);

impl InFlight {
    /// Claim `slot`. `false` means the call must be dropped.
    pub fn begin(&self, slot: &'static str) -> bool {
        let mut open = self.0.borrow_mut();
        if open.contains(&slot) {
            return false;
        }
        open.push(slot);
        true
    }

    pub fn end(&self, slot: &'static str) {
        self.0.borrow_mut().retain(|open| *open != slot);
    }

    pub fn is_open(&self, slot: &str) -> bool {
        self.0.borrow().contains(&slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_second_call_for_the_same_slot_is_refused() {
        let guard = InFlight::default();
        assert!(guard.begin("storage.get"));
        assert!(!guard.begin("storage.get"));
        assert!(guard.begin("storage.set"), "other slots are unaffected");
        guard.end("storage.get");
        assert!(guard.begin("storage.get"));
    }
}
