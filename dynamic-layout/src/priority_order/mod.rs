use std::{cell::RefCell, collections::HashMap, fmt::Debug, rc::Rc};

use dynisland_abi::module::ActivityIdentifier;
use dynisland_core::graphics::activity_widget::ActivityWidget;

pub mod cycle_order;
// pub mod ui_update;

pub trait WidgetOrderManager: Debug + Default {
    fn is_active(&self, id: &ActivityIdentifier) -> bool;
    fn is_shown(&self, id: &ActivityIdentifier) -> bool;
    fn set_container(&mut self, container: gtk::Box);
    fn get_container(&self) -> Option<gtk::Box>;
    fn get_widget_map(&self) -> Rc<RefCell<HashMap<Rc<ActivityIdentifier>, ActivityWidget>>>;
    fn list_activities(&self) -> Vec<Rc<ActivityIdentifier>>;
    fn update_order(&mut self, order: Vec<&ActivityIdentifier>);
    fn update_config(&mut self, max_active: u16, max_shown: u16);
    /// NOTE: `id` is implicitly hidden and deactivated if UiUpdate is None
    fn add(&mut self, id: &ActivityIdentifier, widget: ActivityWidget);
    /// NOTE: `id` is implicitly deactivated and hidden
    fn remove(&mut self, id: &ActivityIdentifier);
    /// NOTE: `id` is implicitly shown and activated
    fn activate(&mut self, id: &ActivityIdentifier);
    /// NOTE: `id` is implicitly deactivated but not hidden
    fn deactivate(&mut self, id: &ActivityIdentifier);
    fn next(&mut self);
    fn previous(&mut self);
}
