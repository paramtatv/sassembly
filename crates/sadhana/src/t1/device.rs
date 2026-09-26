use crate::t1::ast::{Declaration, Type};

use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeviceBinding {
    pub name: String,
    pub address: u64,
    pub layout: Type,
}

pub struct DeviceLoweringContext {
    pub devices: HashMap<String, DeviceBinding>,
}

/// `W-274`: `new()` takes no arguments, so `Default` is the same
/// constructor under the name the language expects. Written rather than
/// allowed, because `clippy::new_without_default` is asking for an
/// interface and not for silence.
impl Default for DeviceLoweringContext {
    fn default() -> Self {
        Self::new()
    }
}

impl DeviceLoweringContext {
    pub fn new() -> Self {
        Self {
            devices: HashMap::new(),
        }
    }

    pub fn lower_declaration(&mut self, decl: &Declaration) {
        if let Declaration::Device {
            name,
            address,
            layout,
        } = decl
        {
            self.devices.insert(
                name.clone(),
                DeviceBinding {
                    name: name.clone(),
                    address: *address,
                    layout: layout.clone(),
                },
            );
        }
    }

    pub fn resolve_device_address(&self, name: &str) -> Option<u64> {
        self.devices.get(name).map(|b| b.address)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::t1::ast::Type;

    #[test]
    fn test_lower_device_declaration() {
        let mut ctx = DeviceLoweringContext::new();

        let decl = Declaration::Device {
            name: "UART".to_string(),
            address: 0x1000_0000,
            layout: Type::Primitive("Struct".to_string()),
        };

        ctx.lower_declaration(&decl);

        assert_eq!(ctx.resolve_device_address("UART"), Some(0x1000_0000));
        assert_eq!(ctx.resolve_device_address("UNKNOWN"), None);
    }
}
