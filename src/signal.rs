//! GDScript's `signal`: a list of handlers a module calls when the platform
//! reports a change.

use std::cell::RefCell;
use std::rc::Rc;

/// Append-only handler list. Like a Godot signal, connecting never replaces an
/// earlier handler and there is no way to disconnect, so connect once during
/// setup.
///
/// Handlers run while the list is borrowed, so a handler must not connect
/// another handler to the same signal.
pub struct Signal<T> {
    slots: Rc<RefCell<Vec<Box<dyn FnMut(T)>>>>,
}

impl<T> Signal<T> {
    pub fn new() -> Self {
        Self {
            slots: Rc::new(RefCell::new(Vec::new())),
        }
    }

    pub fn connect(&self, handler: impl FnMut(T) + 'static) {
        self.slots.borrow_mut().push(Box::new(handler));
    }

    pub fn is_connected(&self) -> bool {
        !self.slots.borrow().is_empty()
    }

    pub fn emit(&self, value: T)
    where
        T: Clone,
    {
        for slot in self.slots.borrow_mut().iter_mut() {
            slot(value.clone());
        }
    }
}

impl<T> Default for Signal<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> Clone for Signal<T> {
    fn clone(&self) -> Self {
        Self {
            slots: Rc::clone(&self.slots),
        }
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
}
