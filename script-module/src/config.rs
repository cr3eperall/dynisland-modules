use dynisland_core::abi::module::ActivityIdentifier;
use dynisland_macro::{MultiWidgetConfig, OptDeserializeConfig};
use serde::Serialize;

#[derive(Debug, Serialize, Clone, MultiWidgetConfig, OptDeserializeConfig)]
pub struct ScriptConfig {
    #[serde(skip_serializing)]
    pub(crate) scrolling: bool,
    #[serde(skip_serializing)]
    pub(crate) scrolling_speed: f32,
    /// if scrolling, it's in pixels, if not, it's in chars
    #[serde(skip_serializing)]
    pub(crate) max_width: i32,
    #[serde(skip_serializing)]
    pub(crate) minimal_image: String,
    #[child_only]
    pub(crate) exec: String,
}

#[allow(clippy::derivable_impls)]
impl Default for ScriptConfig {
    fn default() -> Self {
        Self {
            exec: "echo \"update your config file: see Wiki\"".to_string(),
            minimal_image: String::from("image-missing-symbolic"),
            scrolling: true,
            scrolling_speed: 30.0,
            max_width: 300,
        }
    }
}

pub(crate) fn get_conf_idx(id: &ActivityIdentifier) -> usize {
    id.metadata()
        .additional_metadata()
        .unwrap()
        .split("|")
        .find(|s| s.starts_with("instance="))
        .unwrap()
        .split("=")
        .last()
        .unwrap()
        .parse::<usize>()
        .unwrap()
}
