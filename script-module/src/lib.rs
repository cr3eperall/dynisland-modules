use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait};
use dynisland_core::abi::{
    abi_stable,
    module::{ModuleBuilder, ModuleBuilderRef},
};

pub mod module;
pub mod utils;
pub mod widget;

use module::new;

pub const NAME: &str = "ScriptModule";

#[export_root_module]
fn instantiate_root_module() -> ModuleBuilderRef {
    ModuleBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}