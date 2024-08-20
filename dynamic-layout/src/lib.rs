mod config;
mod layout;
mod priority_order;
mod window_position;

use abi_stable::{export_root_module, prefix_type::PrefixTypeTrait};
use dynisland_abi::layout::{LayoutManagerBuilder, LayoutManagerBuilderRef};
use layout::new;

pub const NAME: &str = "DynamicLayout";

#[export_root_module]
fn instantiate_root_module() -> LayoutManagerBuilderRef {
    LayoutManagerBuilder {
        new,
        name: NAME.into(),
    }
    .leak_into_prefix()
}
