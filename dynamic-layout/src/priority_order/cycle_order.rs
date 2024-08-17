use std::{collections::VecDeque, rc::Rc};

use dynisland_abi::module::ActivityIdentifier;

use crate::layout::DynamicLayoutConfig;

use super::{ui_update::UiUpdate, WidgetOrderManager};

#[derive(Debug, Default)]
pub struct CycleOrder {
    pub(crate) order: VecDeque<Rc<ActivityIdentifier>>,
    pub(crate) active: u16,
    pub(crate) active_offset: u16,
    pub(crate) max_shown: u16,
    pub(crate) max_active: u16,
}
impl CycleOrder {
    pub fn new(config: &DynamicLayoutConfig) -> Self {
        let max_active = config.max_active.min(config.max_activities);
        CycleOrder {
            max_active,
            max_shown: config.max_activities,
            ..Default::default()
        }
    }
    fn is_active(&self, id: &ActivityIdentifier) -> bool {
        let order_idx = match self.order.iter().position(|tid| **tid == *id) {
            Some(idx) => idx,
            None => return false,
        };
        self.active_offset as usize <= order_idx
            && order_idx < (self.active + self.active_offset).into()
    }
    fn is_shown(&self, id: &ActivityIdentifier) -> bool {
        let order_idx = match self.order.iter().position(|tid| **tid == *id) {
            Some(idx) => idx,
            None => return false,
        };
        order_idx < self.max_shown.into()
    }
}

impl WidgetOrderManager for CycleOrder {
    fn update_config_and_reset(&mut self, max_active: u16, max_shown: u16) {
        let max_shown = if max_shown < max_active {
            max_active + 1
        } else {
            max_shown
        };
        self.active_offset = 0;
        self.active = 0;
        self.max_active = max_active;
        self.max_shown = max_shown;
    }

    fn add(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        if self.order.iter().any(|tid| &**tid == id) {
            return update;
        }
        let shared_id = Rc::new(id.clone());
        self.order.push_back(shared_id.clone());

        // // check if it can be active
        // if self.active < self.max_active {
        //     self.active += 1;
        //     update.activate(&shared_id);
        // }

        // check if it can be shown
        if self.order.len() <= self.max_shown.into() {
            update.show(&shared_id);
        }

        // not shown
        update
    }

    fn remove(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        let idx = match self.order.iter().position(|tid| &**tid == id) {
            Some(idx) => idx,
            None => {
                return update;
            }
        };

        // check if it was active
        if self.is_active(id) {
            self.active -= 1;
        } else if idx < self.active_offset as usize {
            //is before active
            self.active_offset -= 1;
        }

        // check if it was shown
        if self.is_shown(id) {
            if let Some(to_show) = self.order.get((self.max_shown - 1).into()) {
                update.show(to_show);
            }
        }
        self.order.remove(idx);
        update
    }

    fn activate(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        let idx = match self.order.iter().position(|tid| &**tid == id) {
            //1
            Some(idx) => idx,
            None => {
                return update;
            }
        };
        if self.is_active(id) {
            return update;
        }
        if self.is_shown(id) {
            if self.active > 0 {
                if idx <= self.active_offset.into() {
                    // it's left of the activated ones
                    if self.active_offset > 0 {
                        let act = self.order.remove(idx).unwrap();
                        self.active_offset -= 1;
                        self.order.insert(self.active_offset.into(), act);
                        if self.active_offset > 0 {
                            update.move_after(
                                self.order.get((self.active_offset - 1).into()).unwrap(),
                            );
                        } else {
                            update.move_to_first();
                        }
                    }
                } else {
                    // it's right of the activated ones
                    let act = self.order.remove(idx).unwrap();
                    self.order
                        .insert((self.active_offset + self.active).into(), act);
                    update.move_after(
                        self.order
                            .get((self.active_offset + self.active - 1).into())
                            .unwrap(),
                    );
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
                update.deactivate(self.order.get((self.active_offset - 1).into()).unwrap());
            } else {
                //left
                update.deactivate(
                    self.order
                        .get((self.active_offset + self.active).into())
                        .unwrap(),
                );
            }
            /* // has the same logic but is easier to understend
            if self.active<self.max_active{
                    if idx<=self.active_offset.into(){
                        // it's left of the activated ones
                        let act=self.order.remove(idx).unwrap();
                        self.active_offset-=1;
                        self.order.insert(self.active_offset.into(), act);
                    }else{
                        // it's right of the activated ones
                        let act=self.order.remove(idx).unwrap();
                        self.order.insert((self.active_offset+self.active).into(), act);
                    }
                // it's near the activated ones
                self.active+=1;
            }else{
                // an activity needs to be deactivated
                if idx<=self.active_offset.into(){
                    // it's left of the activated ones
                    let act=self.order.remove(idx).unwrap();
                    self.active_offset-=1;
                    self.order.insert(self.active_offset.into(), act);
                    update.hide(self.order.get((self.active_offset+self.active).into()).unwrap());
                }else{
                    // it's right of the activated ones
                    let act=self.order.remove(idx).unwrap();
                    self.order.insert((self.active_offset+self.active).into(), act);
                    self.active_offset=+1;
                    update.hide(self.order.get((self.active_offset-1).into()).unwrap());
                }
            }*/
        } else {
            todo!("for now a widget needs to be already shown to be activated");
        }
        update
    }

    fn deactivate(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        let idx = match self.order.iter().position(|tid| &**tid == id) {
            Some(idx) => idx,
            None => {
                return update;
            }
        };
        if !self.is_active(id) {
            return update;
        }
        let dist_to_left = idx - self.active_offset as usize;
        let dist_to_right = self.active as usize - (dist_to_left) - 1;
        if dist_to_left < dist_to_right {
            //move to the left
            let act = self.order.remove(idx).unwrap();
            self.order.insert((self.active_offset).into(), act);
            if self.active_offset > 0 {
                match self.order.get((self.active_offset - 1).into()) {
                    Some(act) => {
                        update.move_after(act);
                    }
                    None => {
                        update.move_to_first();
                    }
                }
            }
            self.active_offset += 1;
        } else {
            //move to the right
            let act = self.order.remove(idx).unwrap();
            self.order
                .insert((self.active_offset + self.active - 1).into(), act);
            if self.active > 1 {
                update.move_after(
                    self.order
                        .get((self.active_offset + self.active - 2).into())
                        .unwrap(),
                );
            }
        }
        self.active -= 1;
        update
    }

    fn next(&mut self) -> Vec<UiUpdate> {
        let mut updates = vec![];
        if self.order.len() <= 1 {
            return updates;
        }
        let act = self.order.pop_front().unwrap();
        let first = act.clone();
        self.order.push_back(act);
        let mut upd = UiUpdate::default();
        let pre_last = self.order.get(self.order.len() - 2).unwrap();
        upd.move_after(pre_last);
        updates.push(upd);
        if self.order.len() >= self.max_shown.into() {
            //need to show and hide
            let mut upd = UiUpdate::default();
            upd.deactivate(&first);
            upd.hide(&first);
            updates.push(upd);
            let mut upd = UiUpdate::default();
            let last = self.order.get((self.max_shown - 1).into()).unwrap();
            upd.show(last);
            updates.push(upd);
        }
        if self.active_offset > 0 {
            // need to deactivate an activity
            let mut upd = UiUpdate::default();
            let before = self.order.get((self.active_offset - 1).into()).unwrap();
            upd.deactivate(before);
            updates.push(upd);
        }
        if self.active > 0 {
            if let Some(after) = self
                .order
                .get((self.active_offset + self.active - 1).into())
            {
                let mut upd = UiUpdate::default();
                upd.activate(after);
                updates.push(upd);
            }
        }
        updates
    }

    fn previous(&mut self) -> Vec<UiUpdate> {
        let mut updates = vec![];
        if self.order.len() <= 1 {
            return updates;
        }
        let last = self.order.pop_back().unwrap();
        let new_first = last.clone();
        self.order.push_front(last);
        let mut upd = UiUpdate::default();
        upd.move_to_first();
        updates.push(upd);
        if self.order.len() > self.max_shown.into() {
            //need to show and hide
            let mut upd = UiUpdate::default();
            let last_shown = self.order.get((self.max_shown).into()).unwrap();
            upd.deactivate(last_shown);
            upd.hide(last_shown);
            updates.push(upd);
            let mut upd = UiUpdate::default();
            upd.show(&new_first);
            updates.push(upd);
        }
        // need to deactivate an activity
        if self.active > 0 {
            let mut upd = UiUpdate::default();
            let before = self.order.get((self.active_offset).into()).unwrap();
            upd.activate(before);
            updates.push(upd);
            let mut upd = UiUpdate::default();
            if let Some(after) = self.order.get((self.active_offset + self.active).into()) {
                upd.deactivate(after);
            } else {
                upd.deactivate(&new_first);
            }
            updates.push(upd);
        }
        updates
    }

    // fn move_left(&mut self, id: &ActivityIdentifier) -> UiUpdate {
    //     todo!()
    // }

    // fn move_right(&mut self, id: &ActivityIdentifier) -> UiUpdate {
    //     todo!()
    // }
}

/* Old implementation
/// add to the back: widget is not shown if there are already max_activities
    ///
    /// NOTE: `id` is implicitly hidden and deactivated if UiUpdate is None
    fn add(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        if self.priority.contains(id) {
            return update
        };
        self.priority.push_back(id.clone());
        self.order.push_back(id.clone());
        if self.priority.len() > self.max_activities.into() {
            // not shown => not active
            return UiUpdate::default();
        }
        // // is shown
        update.show(id);
        if self.active >= self.max_active {
            // is shown but not active
            return update;
        }
        update.activate(id);
        // is shown and active
        self.active += 1;
        update
    }

    /// remove activity and show the next in the priority if this was shown
    ///
    /// NOTE: `id` is implicitly deactivated and hidden
    fn remove(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        let priority_idx = match self.priority.iter().position(|tid| tid == id) {
            Some(idx) => idx,
            None => return update,
        };
        let order_idx = match self.order.iter().position(|tid| tid == id) {
            Some(idx) => idx,
            None => return update,
        };
        self.priority.remove(priority_idx);
        self.order.remove(order_idx);
        if priority_idx < self.active.into() {
            // was active
            // update.deactivate(id);
            self.active -= 1;
        }
        if priority_idx < self.max_activities.into() {
            // was shown
            // so show next in priority
            if let Some(a) = self.priority.get((self.max_activities-1).into()) {
                update.show(a);
            }
        }
        update
    }

    ///
    ///
    /// NOTE: `id` is implicitly shown and activated
    fn activate(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let mut update = UiUpdate::default();
        let priority=&mut self.priority;
        let priority_idx = match priority.iter().position(|tid| tid == id) {
            Some(idx) => idx,
            None => return update,
        };
        if priority_idx < self.active.into() {
            //already active
            return update;
        }
        // wasn't active
        if self.active<self.max_active{
            // there is still space for an active widget without deactivating another
            self.active += 1;
            let idx=priority.remove(priority_idx).unwrap();
            priority.push_front(idx);
            return update;
        }
        // a widget needs to be deactivated
        if priority_idx >= self.max_activities.into(){
            // was hidden
            if let Some(a) = priority.get((self.max_activities-1).into()) {
                update.hide(a)
            }
            update.deactivate(priority.get((self.active-1).into()).unwrap());
            // put at the top
            let idx=priority.remove(priority_idx).unwrap();
            priority.push_front(idx);
        } else {
            // was shown

            if let Some(a) = priority.get(priority_idx) {
                update.deactivate(a)
            }
        }

        update
    }

    /// NOTE: doesn't activate the widget that goes up the priority
    ///
    /// NOTE: `id` is implicitly deactivated but not hidden
    fn deactivate(&mut self, id: &ActivityIdentifier) -> UiUpdate {
        let update = UiUpdate::default();
        let priority_idx = match self.priority.iter().position(|tid| tid == id) {
            Some(idx) => idx,
            None => return update,
        };
        if priority_idx >= self.active.into() {
            // wasn't active
        }else {
            // was active
            let idx=self.priority.remove(priority_idx).unwrap();
            self.active-=1;
            self.priority.insert(self.max_active.into(), idx);
        }
        update
    }

    fn update_config(&mut self, config: &DynamicLayoutConfig) -> Vec<UiUpdate> {
        let update = vec![];
        let max_activities = if config.max_active>=config.max_activities{
            config.max_active+1
        }else{
            config.max_activities
        };
        self.max_activities=max_activities;
        self.max_active=config.max_active;
        self.active=self.active.min(self.max_active);
        update
    }
    */
