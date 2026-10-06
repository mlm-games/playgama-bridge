use std::rc::Rc;

use crate::api::DeviceApi;
use crate::js::Modules;
use crate::types::DeviceType;

pub(crate) struct Device {
    module: crate::js::Module,
}

impl Device {
    pub(crate) fn attach(modules: &Modules) -> Option<Rc<dyn DeviceApi>> {
        Some(Rc::new(Self {
            module: modules.get("device")?,
        }))
    }
}

impl DeviceApi for Device {
    fn device_type(&self) -> DeviceType {
        self.module.text("type").unwrap_or(DeviceType::Desktop)
    }
}
