use std::{
    cell::RefCell,
    collections::{HashMap, HashSet, VecDeque},
    rc::Rc,
};

use dynisland_core::{
    abi::{gdk, glib, gtk, module::ActivityIdentifier},
    graphics::activity_widget::{boxed_activity_mode::ActivityMode, ActivityWidget},
};
use gdk::prelude::ListModelExtManual;
use gtk::prelude::*;

use crate::config::DynamicLayoutConfig;

#[derive(Debug)]
pub struct CycleOrder {
    pub(crate) window: gtk::Window,
    pub(crate) container: gtk::Box,
    pub(crate) widget_map: Rc<RefCell<HashMap<Rc<ActivityIdentifier>, ActivityWidget>>>,
    pub(crate) order: VecDeque<Rc<ActivityIdentifier>>,
    pub(crate) active: u16,
    pub(crate) active_offset: u16,
    pub(crate) max_shown: u16,
    pub(crate) max_active: u16,
}
impl CycleOrder {
    pub fn new(config: &DynamicLayoutConfig, window: &gtk::Window, container: &gtk::Box) -> Self {
        let max_active = config.max_active.min(config.max_activities);
        CycleOrder {
            max_active,
            max_shown: config.max_activities,
            container: container.clone(),
            window: window.clone(),
            widget_map: Rc::new(RefCell::new(HashMap::new())),
            order: VecDeque::new(),
            active: 0,
            active_offset: 0,
        }
    }
    fn update_ui(&self) {
        //remove widgets
        if self.container.clone().first_child().is_none() && self.widget_map.borrow().is_empty() {
            return;
        }

        let all_widgets = self
            .widget_map
            .borrow()
            .values()
            .cloned()
            .collect::<HashSet<ActivityWidget>>();
        let mut to_remove = Vec::new();
        let mut container_children = HashSet::new();
        for widget in self
            .container
            .clone()
            .observe_children()
            .iter::<glib::Object>()
            .flatten()
        {
            let widget = widget.downcast::<ActivityWidget>().unwrap();
            container_children.insert(widget.clone());
            if !all_widgets.contains(&widget) {
                to_remove.push(widget);
            }
        }
        for widget in to_remove {
            self.container.clone().remove(&widget);
        }

        //add widgets
        for widget in all_widgets {
            if !container_children.contains(&widget) {
                self.container.clone().append(&widget);
            }
        }

        //reorder and activate/deactivate widgets
        let mut last_widget = None::<&ActivityWidget>;
        let widget_map = self.widget_map.borrow();
        for widget_id in self.order.iter() {
            let widget = widget_map.get(widget_id.as_ref()).unwrap();
            self.container
                .clone()
                .reorder_child_after(widget, last_widget);

            if self.is_active(widget_id) {
                widget.set_mode(ActivityMode::Compact);
            } else {
                widget.set_mode(ActivityMode::Minimal);
            }
            if self.is_shown(widget_id) {
                widget.set_visible(true);
                widget.remove_css_class("hidden");
            } else {
                widget.add_css_class("hidden");
            }
            last_widget = Some(widget);
        }
    }
}

impl CycleOrder {
    pub fn is_active(&self, id: &ActivityIdentifier) -> bool {
        if let Some(pos) = self.order.iter().position(|t| t.as_ref() == id) {
            (pos as u16) >= self.active_offset && (pos as u16) < self.active + self.active_offset
        } else {
            false
        }
    }

    pub fn is_shown(&self, id: &ActivityIdentifier) -> bool {
        if let Some(pos) = self.order.iter().position(|t| t.as_ref() == id) {
            (pos as u16) < self.max_shown
        } else {
            false
        }
    }

    pub fn get_container(&self) -> gtk::Box {
        self.container.clone()
    }

    pub fn get_widget_map(&self) -> Rc<RefCell<HashMap<Rc<ActivityIdentifier>, ActivityWidget>>> {
        self.widget_map.clone()
    }
    pub fn get_window(&self) -> gtk::Window {
        self.window.clone()
    }

    pub fn list_activities(&self) -> Vec<Rc<ActivityIdentifier>> {
        self.order.iter().cloned().collect()
    }

    pub fn update_order(&mut self, order: Vec<&ActivityIdentifier>) {
        if self.order.len() != order.len() {
            return;
        }
        let mut self_ordered = self
            .order
            .iter()
            .map(|s| s.as_ref())
            .collect::<Vec<&ActivityIdentifier>>();
        self_ordered.sort();
        let mut other_ordered = order.clone();
        other_ordered.sort();
        for (i, widget) in self_ordered.iter().enumerate() {
            if widget != &other_ordered[i] {
                return;
            }
        }

        self.order.clear();
        for id in order {
            self.order.push_back(Rc::new(id.clone()));
        }
        self.update_ui();
    }

    pub fn update_config(&mut self, max_active: u16, max_shown: u16) {
        let max_shown = max_shown.max(max_active);
        self.max_active = max_active;
        self.max_shown = max_shown;
        self.active = self.active.min(max_active);

        self.update_ui();
    }

    pub fn add(&mut self, id: &ActivityIdentifier, widget: ActivityWidget) {
        if self.widget_map.borrow().contains_key(id) {
            return;
        }
        self.widget_map
            .borrow_mut()
            .insert(Rc::new(id.clone()), widget);
        let shared_id = Rc::new(id.clone());
        self.order.push_back(shared_id.clone());
        self.update_ui();
    }

    pub fn remove(&mut self, id: &ActivityIdentifier) {
        let postion = self.order.iter().position(|tid| tid.as_ref() == id);
        if postion.is_none() {
            return;
        }
        if self.is_active(id) {
            self.active -= 1;
        } else if postion.unwrap() < self.active_offset as usize {
            self.active_offset -= 1;
        }
        self.widget_map.borrow_mut().remove(id);
        self.order.remove(postion.unwrap());
        self.update_ui();
    }

    pub fn activate(&mut self, id: &ActivityIdentifier) {
        if self.is_active(id) {
            return;
        }
        let idx = match self.order.iter().position(|tid| tid.as_ref() == id) {
            Some(idx) => idx,
            None => {
                return;
            }
        };
        if !self.is_shown(id) {
            let act = self.order.remove(idx).unwrap();
            self.order
                .insert((self.max_shown as usize).min(self.order.len()) - 1, act);
        }
        if self.active > 0 {
            let act = self.order.remove(idx).unwrap();
            if idx <= self.active_offset.into() {
                // it's left of the activated ones
                self.active_offset -= 1;
                self.order.insert(self.active_offset.into(), act);
            } else {
                // it's right of the activated ones
                self.order
                    .insert((self.active_offset + self.active).into(), act);
            }
        } else {
            self.active_offset = idx as u16;
        }
        // check if there is still space for active
        if self.active < self.max_active {
            self.active += 1;
        } else if idx > self.active_offset.into() {
            //right
            self.active_offset += 1;
        }
        self.update_ui();
    }

    pub fn deactivate(&mut self, id: &ActivityIdentifier) {
        if !self.is_active(id) {
            return;
        }
        let idx = match self.order.iter().position(|tid| tid.as_ref() == id) {
            Some(idx) => idx,
            None => {
                return;
            }
        };

        let dist_to_left = idx - self.active_offset as usize;
        let dist_to_right = self.active as usize - (dist_to_left) - 1;
        if dist_to_left < dist_to_right {
            //move to the left
            let act = self.order.remove(idx).unwrap();
            self.order.insert((self.active_offset).into(), act);
            self.active_offset += 1;
        } else {
            //move to the right
            let act = self.order.remove(idx).unwrap();
            self.order
                .insert((self.active_offset + self.active - 1).into(), act);
        }
        self.active -= 1;
        self.update_ui();
    }

    pub fn next(&mut self) {
        if let Some(back) = self.order.pop_back() {
            self.order.push_front(back);
            self.update_ui();
        }
    }

    pub fn previous(&mut self) {
        if let Some(front) = self.order.pop_front() {
            self.order.push_back(front);
            self.update_ui();
        }
    }
}
