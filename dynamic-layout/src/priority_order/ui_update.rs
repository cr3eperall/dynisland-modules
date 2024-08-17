use std::{fmt::Display, rc::Rc};

use dynisland_abi::module::ActivityIdentifier;

#[derive(Debug, Default)]
pub struct UiUpdate {
    pub(crate) activate: bool,
    pub(crate) activate_widget: Option<Rc<ActivityIdentifier>>,
    pub(crate) show: bool,
    pub(crate) show_widget: Option<Rc<ActivityIdentifier>>,
    pub(crate) to_move_this: bool,
    pub(crate) to_move_this_after: Option<Rc<ActivityIdentifier>>,
}

pub struct AllResult {
    pub to_activate: Option<Rc<ActivityIdentifier>>,
    pub to_deactivate: Option<Rc<ActivityIdentifier>>,
    pub to_show: Option<Rc<ActivityIdentifier>>,
    pub to_hide: Option<Rc<ActivityIdentifier>>,
    pub move_this: bool,
    pub to_move_this_after: Option<Rc<ActivityIdentifier>>,
}

impl UiUpdate {
    pub fn is_empty(&self) -> bool {
        self.activate_widget.is_none() && self.show_widget.is_none() && !self.to_move_this
    }

    pub(crate) fn activate(&mut self, activity: &Rc<ActivityIdentifier>) {
        self.activate = true;
        self.activate_widget = Some(activity.clone())
    }
    pub(crate) fn deactivate(&mut self, activity: &Rc<ActivityIdentifier>) {
        self.activate = false;
        self.activate_widget = Some(activity.clone())
    }
    pub(crate) fn show(&mut self, activity: &Rc<ActivityIdentifier>) {
        self.show = true;
        self.show_widget = Some(activity.clone())
    }
    pub(crate) fn hide(&mut self, activity: &Rc<ActivityIdentifier>) {
        self.show = false;
        self.show_widget = Some(activity.clone())
    }
    pub(crate) fn move_after(&mut self, activity: &Rc<ActivityIdentifier>) {
        self.to_move_this = true;
        self.to_move_this_after = Some(activity.clone());
    }
    pub(crate) fn move_to_first(&mut self) {
        self.to_move_this = true;
        self.to_move_this_after = None;
    }

    /// Returns (to_activate, to_deactivate, to_show, to_hide) without cloning
    pub fn all(mut self) -> AllResult {
        let (to_activate, to_deactivate) = if self.activate {
            (self.activate_widget.take(), None)
        } else {
            (None, self.activate_widget.take())
        };
        let (to_show, to_hide) = if self.show {
            (self.show_widget.take(), None)
        } else {
            (None, self.show_widget.take())
        };
        AllResult {
            to_activate,
            to_deactivate,
            to_show,
            to_hide,
            move_this: self.to_move_this,
            to_move_this_after: self.to_move_this_after,
        }
    }
}

impl Display for UiUpdate {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(act) = self.activate_widget.clone() {
            let word = if self.activate {
                "activate"
            } else {
                "deactivate"
            };
            write!(f, "{word} {act}, ")?;
        }
        if let Some(act) = self.show_widget.clone() {
            let word = if self.activate { "show" } else { "hide" };
            write!(f, "{word} {act}, ")?;
        }
        if self.to_move_this {
            if let Some(after) = self.to_move_this_after.clone() {
                write!(f, "move this after {after}")?;
            } else {
                write!(f, "move this to first")?;
            };
        }
        Ok(())
    }
}
