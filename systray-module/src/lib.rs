#[cfg(not(feature = "embedded"))]
use abi_stable::export_root_module;
use abi_stable::prefix_type::PrefixTypeTrait;
use dynisland_core::abi::module::{ModuleBuilder, ModuleBuilderRef};

mod config;
mod item_menu_tasks;
mod item_tasks;
mod module;
mod status_notifier;
mod widget;

use module::new;

pub const NAME: &str = "SystrayModule";

#[cfg_attr(not(feature = "embedded"), export_root_module)]
pub fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
