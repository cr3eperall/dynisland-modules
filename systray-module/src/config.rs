use dynisland_core::d_macro::{MultiWidgetConfig, OptDeserializeConfig};
use serde::Serialize;

#[derive(Debug, Serialize, MultiWidgetConfig, OptDeserializeConfig, Clone)]
#[serde(default)]
pub struct SystrayConfig {
    pub(crate) template_field: String,
}

#[allow(clippy::derivable_impls)]
impl Default for SystrayConfig {
    fn default() -> Self {
        Self {
            template_field: String::from("default"),
        }
    }
}
