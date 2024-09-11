use dynisland_core::{
    abi::module::ActivityIdentifier,
    d_macro::{MultiWidgetConfig, OptDeserializeConfig},
};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, MultiWidgetConfig, OptDeserializeConfig)]
pub struct ExampleConfig {
    pub(crate) int: i32,
    pub(crate) string: String,
    pub(crate) vec: Vec<String>,
    pub(crate) duration: u64,
}

impl Default for ExampleConfig {
    fn default() -> Self {
        Self {
            int: 0,
            string: String::from("Example1"),
            vec: vec![String::from("Example2"), String::from("Example3")],
            duration: 400,
        }
    }
}

pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata("instance")
        .unwrap()
        .parse::<usize>()
        .unwrap()
}
