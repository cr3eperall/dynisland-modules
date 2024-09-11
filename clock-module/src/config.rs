use dynisland_core::{
    abi::module::ActivityIdentifier,
    d_macro::{MultiWidgetConfig, OptDeserializeConfig},
};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, MultiWidgetConfig, OptDeserializeConfig)]
pub struct ClockConfig {
    pub(crate) format_24h: bool,
    pub(crate) hour_hand_color: String,
    pub(crate) minute_hand_color: String,
    pub(crate) tick_color: String,
    pub(crate) circle_color: String,
}

impl Default for ClockConfig {
    fn default() -> Self {
        Self {
            format_24h: true,
            hour_hand_color: String::from("white"),
            minute_hand_color: String::from("white"),
            circle_color: String::from("lightgray"),
            tick_color: String::from("lightgray"),
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
