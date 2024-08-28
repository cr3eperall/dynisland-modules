mod config;
mod layout;
mod priority_order;
mod window_position;
use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait};
use dynisland_core::abi::{
    abi_stable,
    layout::{LayoutManagerBuilder, LayoutManagerBuilderRef},
};
use layout::new;

pub const NAME: &str = "DynamicLayout";

#[cfg_attr(not(feature = "embedded"), export_root_module)]
pub fn instantiate_root_module() -> LayoutManagerBuilderRef {
    LayoutManagerBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
