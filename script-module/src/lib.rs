#[cfg(not(feature = "embedded"))]
use abi_stable::export_root_module;
use abi_stable::prefix_type::PrefixTypeTrait;
use dynisland_core::abi::{
    abi_stable,
    module::{ModuleBuilder, ModuleBuilderRef},
};

pub mod module;
pub mod utils;
pub mod widget;

use module::new;

pub const NAME: &str = "ScriptModule";

#[cfg_attr(not(feature = "embedded"), export_root_module)]
pub fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
