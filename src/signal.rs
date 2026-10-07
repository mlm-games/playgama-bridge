//! GDScript's `signal`: a list of handlers a module calls when the platform
//! reports a change.

use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::Rc;

/// Append-only handler list. Like a Godot signal, connecting never replaces an
/// earlier handler and there is no way to disconnect, so connect once during
/// setup.
///
/// A handler may emit the same signal again. The SDK does exactly that, when
/// `hideBanner` fires `banner_state_changed` from inside a listener. That value
/// is queued and delivered once the current pass finishes, because a `FnMut`
/// handler cannot be on the stack twice.
#[derive(Clone)]
pub struct Signal<T> {
    slots: Rc<RefCell<Vec<Handler<T>>>>,
    queued: Rc<RefCell<VecDeque<T>>>,
    delivering: Cell<bool>,
}

/// A handler in a slot of its own, so a handler may `connect` another one or ask
/// whether anything is connected while it runs.
type Handler<T> = Rc<RefCell<Option<Box<dyn FnMut(T)>>>>;

impl<T> Signal<T> {
    pub fn new() -> Self {
        Self {
            slots: Rc::new(RefCell::new(Vec::new())),
            queued: Rc::new(RefCell::new(VecDeque::new())),
            delivering: Cell::new(false),
        }
    }

    pub fn connect(&self, handler: impl FnMut(T) + 'static) {
        self.slots
            .borrow_mut()
            .push(Rc::new(RefCell::new(Some(Box::new(handler)))));
    }

    pub fn is_connected(&self) -> bool {
        self.slots
            .borrow()
            .iter()
            .any(|handler| handler.borrow().is_some())
    }

    pub fn emit(&self, value: T)
    where
        T: Clone,
    {
        if self.delivering.replace(true) {
            self.queued.borrow_mut().push_back(value);
            return;
        }
        let mut next = Some(value);
        while let Some(value) = next {
            // The list is cloned out before any handler runs, so a handler
            // connected during this pass takes effect from the next value on.
            let handlers = self.slots.borrow().clone();
            for handler in handlers {
                let Some(mut call) = handler.borrow_mut().take() else {
                    continue;
                };
                call(value.clone());
                if handler.borrow().is_none() {
                    *handler.borrow_mut() = Some(call);
                }
            }
            next = self.queued.borrow_mut().pop_front();
        }
        self.delivering.set(false);
    }
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    #[test]
    fn every_handler_sees_every_emit() {
        let signal = Signal::new();
        let seen = Rc::new(RefCell::new(Vec::new()));
        let a = Rc::clone(&seen);
        signal.connect(move |v: i32| a.borrow_mut().push(v));
        let b = Rc::clone(&seen);
        signal.connect(move |v: i32| b.borrow_mut().push(v * 10));
        assert!(signal.is_connected());
        signal.emit(1);
        signal.emit(2);
        assert_eq!(*seen.borrow(), vec![1, 10, 2, 20]);
    }

    #[test]
    fn a_clone_shares_the_handler_list() {
        let signal = Signal::new();
        let clone = signal.clone();
        let seen = Rc::new(Cell::new(0));
        let counter = Rc::clone(&seen);
        clone.connect(move |_: bool| counter.set(counter.get() + 1));
        signal.emit(true);
        assert_eq!(seen.get(), 1);
    }

    /// The SDK emits synchronously from inside a listener, so a "hide the banner
    /// when it failed" handler re-enters its own signal.
    #[test]
    fn a_handler_may_emit_and_connect_on_its_own_signal() {
        let signal = Rc::new(Signal::new());
        let seen = Rc::new(RefCell::new(Vec::new()));
        let log = Rc::clone(&seen);
        let nested = Rc::clone(&signal);
        let inner_log = Rc::clone(&seen);
        let connect_log = Rc::clone(&inner_log);
        signal.connect(move |v: i32| {
            log.borrow_mut().push(v);
            if v == 1 {
                nested.emit(2);
                let sink = Rc::clone(&connect_log);
                nested.connect(move |w: i32| sink.borrow_mut().push(w * 100));
            }
        });
        signal.emit(1);
        signal.emit(3);
        // 2 is queued behind the pass that was running, so the new handler sees
        // it; the outer 3 then reaches both handlers.
        assert_eq!(*seen.borrow(), vec![1, 2, 200, 3, 300]);
    }
}
