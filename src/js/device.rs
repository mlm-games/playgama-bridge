use std::rc::Rc;

use crate::api::DeviceApi;
use crate::js::{Listeners, Module, Modules};
use crate::signal::Signal;
use crate::types::{DeviceOrientation, DeviceOs, DeviceType, SafeArea, ScreenSize};

pub(crate) struct Device {
    module: Module,
    listeners: Listeners,
    orientation: Signal<Option<DeviceOrientation>>,
    screen_size: Signal<Option<ScreenSize>>,
}

impl Device {
    pub(crate) fn attach(modules: &Modules) -> Option<Rc<dyn DeviceApi>> {
        let module = modules.get("device")?;
        let device = Self {
            module: module.clone(),
            listeners: Listeners::default(),
            orientation: Signal::new(),
            screen_size: Signal::new(),
        };
        device.listeners.listen(
            "orientation_state_changed",
            |value| value.as_string().and_then(|text| text.parse().ok()),
            &device.orientation,
        );
        // The payload is `{ width, height }`; anything else is not a size we
        // can lay out against.
        device.listeners.listen(
            "screen_size_changed",
            |value| {
                let size = crate::convert::from_js(value);
                Some(ScreenSize {
                    width: size.get("width")?.as_f64()?,
                    height: size.get("height")?.as_f64()?,
                })
            },
            &device.screen_size,
        );
        Some(Rc::new(device))
    }
}

impl DeviceApi for Device {
    fn device_type(&self) -> DeviceType {
        self.module.text("type").unwrap_or(DeviceType::Desktop)
    }

    fn os(&self) -> DeviceOs {
        self.module.text("os").unwrap_or(DeviceOs::Other)
    }

    fn device_type_text(&self) -> Option<String> {
        self.module.string("type")
    }

    fn orientation(&self) -> Option<DeviceOrientation> {
        self.module.text("orientation")
    }

    fn safe_area(&self) -> SafeArea {
        self.module.safe_area("safeArea")
    }

    fn orientation_state_changed(&self) -> &Signal<Option<DeviceOrientation>> {
        &self.orientation
    }

    fn screen_size_changed(&self) -> &Signal<Option<ScreenSize>> {
        &self.screen_size
    }
}
