#[cfg(not(feature = "embedded"))]
use abi_stable::export_root_module;
use abi_stable::prefix_type::PrefixTypeTrait;
use dynisland_core::abi::module::{ModuleBuilder, ModuleBuilderRef};

pub mod config;
pub mod module;
pub mod player_info;
pub mod producer_tasks;
pub mod utils;
pub mod widget;

use module::new;

pub const NAME: &str = "MusicModule";

#[cfg_attr(not(feature = "embedded"), export_root_module)]
pub fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
