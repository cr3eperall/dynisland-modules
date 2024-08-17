use std::fmt::Debug;

use dynisland_abi::module::ActivityIdentifier;
use ui_update::UiUpdate;

pub mod cycle_order;
pub mod ui_update;

pub trait WidgetOrderManager: Debug + Default {
    fn update_config_and_reset(&mut self, max_active: u16, max_shown: u16);
    /// NOTE: `id` is implicitly hidden and deactivated if UiUpdate is None
    fn add(&mut self, id: &ActivityIdentifier) -> UiUpdate;
    /// NOTE: `id` is implicitly deactivated and hidden
    fn remove(&mut self, id: &ActivityIdentifier) -> UiUpdate;
    /// NOTE: `id` is implicitly shown and activated
    fn activate(&mut self, id: &ActivityIdentifier) -> UiUpdate;
    /// NOTE: `id` is implicitly deactivated but not hidden
    fn deactivate(&mut self, id: &ActivityIdentifier) -> UiUpdate;
    fn next(&mut self) -> Vec<UiUpdate>;
    fn previous(&mut self) -> Vec<UiUpdate>;
    // fn move_left(&mut self, id: &ActivityIdentifier) -> UiUpdate;
    // fn move_right(&mut self, id: &ActivityIdentifier) -> UiUpdate;
}
