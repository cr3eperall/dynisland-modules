use std::{cell::RefCell, collections::HashMap, sync::Arc};

use dynisland_core::{
    abi::{
        gdk, glib,
        gtk::{self, EventSequenceState},
    },
    graphics::widgets::rolling_char::RollingChar,
};
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object,
};
use gtk::{
    prelude::*,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use crate::status_notifier::item::Status;

/// tooltip is a markup string, only supports `<b>`, `<i>` and `<u>` tags, other tags need to be removed by the caller
pub struct ItemData {
    pub tooltip: String,
    pub status: Status,
    pub icon: gdk::Paintable,
    pub attention_icon: Option<gdk::Paintable>,
    pub overlay_icon: Option<gdk::Paintable>,
}

glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}
#[derive(Debug, Clone, Copy)]
pub enum ItemAction {
    Clicked(u32),
    Scrolled(bool, i32),
}

#[derive(CompositeTemplate)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/systrayModule/compact.ui")]
pub struct CompactPriv {
    pub items: RefCell<HashMap<String, (gtk::Overlay, ItemData)>>,
    pub action_tx: RefCell<UnboundedSender<(String, ItemAction)>>,
    pub action_rx: Arc<Mutex<UnboundedReceiver<(String, ItemAction)>>>,
    #[template_child]
    pub container: TemplateChild<gtk::Box>,
}

impl Default for CompactPriv {
    fn default() -> Self {
        let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            items: RefCell::new(HashMap::new()),
            action_tx: RefCell::new(action_tx),
            action_rx: Arc::new(Mutex::new(action_rx)),
            container: TemplateChild::default(),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "SystrayCompactWidget";
    type Type = Compact;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        RollingChar::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        // If you use template callbacks (for example running a function when a button is pressed), uncomment this
        // klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for CompactPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}

impl WidgetImpl for CompactPriv {}

impl Compact {
    pub fn new() -> Self {
        let this: Self = Object::builder().build();

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        // let _ = activity.add_dynamic_property("roll-char", '0');

        // let minimal = this.clone();
        // activity
        //     .subscribe_to_property("roll-char", move |new_value| {
        //         let value_char = cast_dyn_any!(new_value, char).unwrap();
        //         log::trace!("char changed: {value_char}");
        //         minimal.imp().roll.set_current_char(value_char);
        //     })
        //     .unwrap();

        this
    }

    pub fn insert_item(&self, id: &str, data: ItemData) -> bool {
        let mut items = self.imp().items.borrow_mut();
        let (overlay, image) = if let Some((overlay, _)) = items.get(id) {
            (
                overlay.clone(),
                overlay.child().unwrap().downcast::<gtk::Image>().unwrap(),
            )
        } else {
            let image = gtk::Image::builder()
                .width_request(30)
                .height_request(30)
                .css_classes(vec!["icon"])
                .build();
            let overlay = gtk::Overlay::builder()
                .child(&image)
                .css_classes(["icon"])
                .name(id)
                .build();
            let id1 = id.to_string();
            let action_tx = self.imp().action_tx.borrow().clone();
            let primary_gest = gtk::GestureClick::new();
            primary_gest.set_button(gdk::BUTTON_PRIMARY);
            primary_gest.connect_released(move |gest, _n, x, y| {
                let wid = gest.widget();
                if x < 0.0
                    || y < 0.0
                    || x > wid.size(gtk::Orientation::Horizontal).into()
                    || y > wid.size(gtk::Orientation::Vertical).into()
                {
                    return;
                }
                let button = gest.current_button();
                action_tx
                    .send((id1.clone(), ItemAction::Clicked(button)))
                    .unwrap();

                gest.set_state(EventSequenceState::Claimed);
            });
            overlay.add_controller(primary_gest);
            let id1 = id.to_string();
            let action_tx = self.imp().action_tx.borrow().clone();
            let secondary_gest = gtk::GestureClick::new();
            secondary_gest.set_button(gdk::BUTTON_SECONDARY);
            secondary_gest.connect_released(move |gest, _n, x, y| {
                let wid = gest.widget();
                if x < 0.0
                    || y < 0.0
                    || x > wid.size(gtk::Orientation::Horizontal).into()
                    || y > wid.size(gtk::Orientation::Vertical).into()
                {
                    return;
                }
                let button = gest.current_button();
                action_tx
                    .send((id1.clone(), ItemAction::Clicked(button)))
                    .unwrap();

                gest.set_state(EventSequenceState::Claimed);
            });
            overlay.add_controller(secondary_gest);
            let id1 = id.to_string();
            let action_tx = self.imp().action_tx.borrow().clone();
            let middle_gest = gtk::GestureClick::new();
            middle_gest.set_button(gdk::BUTTON_MIDDLE);
            middle_gest.connect_released(move |gest, _n, x, y| {
                let wid = gest.widget();
                if x < 0.0
                    || y < 0.0
                    || x > wid.size(gtk::Orientation::Horizontal).into()
                    || y > wid.size(gtk::Orientation::Vertical).into()
                {
                    return;
                }
                let button = gest.current_button();
                action_tx
                    .send((id1.clone(), ItemAction::Clicked(button)))
                    .unwrap();

                gest.set_state(EventSequenceState::Claimed);
            });
            overlay.add_controller(middle_gest);
            let id1 = id.to_string();
            let action_tx = self.imp().action_tx.borrow().clone();
            let scroll_gest = gtk::EventControllerScroll::new(
                gtk::EventControllerScrollFlags::BOTH_AXES
                    | gtk::EventControllerScrollFlags::DISCRETE,
            );
            scroll_gest.connect_scroll(move |_gest, dx, dy| {
                if dy != 0.0 {
                    action_tx
                        .send((id1.clone(), ItemAction::Scrolled(true, dy as i32)))
                        .unwrap();
                }
                if dx != 0.0 {
                    action_tx
                        .send((id1.clone(), ItemAction::Scrolled(false, dx as i32)))
                        .unwrap();
                }

                glib::Propagation::Proceed
            });
            (overlay, image)
        };
        match data.status {
            Status::Passive => {
                image.set_paintable(Some(&data.icon));
            }
            Status::Active => {
                image.set_paintable(Some(&data.icon));
            }
            Status::NeedsAttention => {
                if let Some(attention_icon) = &data.attention_icon {
                    image.set_paintable(Some(attention_icon));
                }
            }
        }
        if let Some(ov) = overlay.last_child() {
            if !ov.has_css_class("icon") {
                overlay.remove_overlay(&ov);
            }
        }
        if let Some(overlay_icon) = &data.overlay_icon {
            let overlay_image = gtk::Image::builder().paintable(overlay_icon).build();
            overlay.add_overlay(&overlay_image);
        }
        if !data.tooltip.is_empty() {
            overlay.set_tooltip_markup(Some(&data.tooltip));
        } else {
            overlay.set_tooltip_markup(None);
        }
        if let None = items.insert(id.to_string(), (overlay.clone(), data)) {
            self.imp().container.append(&overlay);
            true
        } else {
            false
        }
    }

    pub fn remove_item(&self, id: &str) -> bool {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, _)) = items.remove(id) {
            self.imp().container.remove(&overlay);
            true
        } else {
            false
        }
    }

    pub fn update_item_status(&self, id: &str, status: Status) {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, data)) = items.get_mut(id) {
            data.status = status;
            let image = overlay.child().unwrap().downcast::<gtk::Image>().unwrap();
            match data.status {
                Status::Passive => {
                    image.set_paintable(Some(&data.icon));
                }
                Status::Active => {
                    image.set_paintable(Some(&data.icon));
                }
                Status::NeedsAttention => {
                    if let Some(attention_icon) = &data.attention_icon {
                        image.set_paintable(Some(attention_icon));
                    }
                }
            }
        }
    }

    pub fn update_item_tooltip(&self, id: &str, tooltip: &str) {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, data)) = items.get_mut(id) {
            data.tooltip = tooltip.to_string();
            if !data.tooltip.is_empty() {
                overlay.set_tooltip_markup(Some(&data.tooltip));
            } else {
                overlay.set_tooltip_markup(None);
            }
        }
    }

    pub fn update_item_icon(&self, id: &str, icon: gdk::Paintable) {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, data)) = items.get_mut(id) {
            data.icon = icon;
            let image = overlay.child().unwrap().downcast::<gtk::Image>().unwrap();
            match data.status {
                Status::Passive | Status::Active => {
                    image.set_paintable(Some(&data.icon));
                }
                Status::NeedsAttention => {
                    if data.attention_icon.is_none() {
                        image.set_paintable(Some(&data.icon));
                    }
                }
            }
        }
    }

    pub fn update_item_attention_icon(&self, id: &str, icon: Option<gdk::Paintable>) {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, data)) = items.get_mut(id) {
            data.attention_icon = icon;
            let image = overlay.child().unwrap().downcast::<gtk::Image>().unwrap();
            match data.status {
                Status::NeedsAttention => {
                    if let Some(attention_icon) = &data.attention_icon {
                        image.set_paintable(Some(attention_icon));
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update_item_overlay_icon(&self, id: &str, icon: Option<gdk::Paintable>) {
        let mut items = self.imp().items.borrow_mut();
        if let Some((overlay, data)) = items.get_mut(id) {
            data.overlay_icon = icon;
            if let Some(ov) = overlay.last_child() {
                if !ov.has_css_class("icon") {
                    overlay.remove_overlay(&ov);
                }
            }
            if let Some(overlay_icon) = &data.overlay_icon {
                let overlay_image = gtk::Image::builder().paintable(overlay_icon).build();
                overlay.add_overlay(&overlay_image);
            }
        }
    }

    pub fn clear_items(&self) {
        let mut items = self.imp().items.borrow_mut();
        items.clear();
        let mut to_remove = Vec::new();
        for widget in self
            .imp()
            .container
            .observe_children()
            .iter::<glib::Object>()
            .flatten()
        {
            let widget = widget.downcast::<gtk::Widget>().unwrap();
            to_remove.push(widget.clone());
        }
        for widget in to_remove {
            self.imp().container.remove(&widget);
        }
    }
}
