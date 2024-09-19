use std::{borrow::Borrow, cell::RefCell};

use dynisland_core::abi::{
    gdk::{self, Paintable},
    glib::{self, derived_properties, Properties},
    gtk::{
        self,
        gdk::{prelude::*, subclass::prelude::*},
        prelude::*,
        EventControllerMotion, GestureClick,
    },
};
use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    Object,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};
use tokio::sync::mpsc::UnboundedSender;

use crate::status_notifier::{
    icon,
    layout::{self, LayoutChild, LayoutProperty, ToggleProperty, TypeProperty},
};

#[derive(Debug, Clone, Copy)]
pub enum MenuItemAction {
    Clicked(i32),
    Hovered(i32),
    OpenMenu(i32),
    GoBack,
}

glib::wrapper! {
    pub struct MenuItem(ObjectSubclass<MenuItemPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties, Default)]
#[properties(wrapper_type = MenuItem)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/systrayModule/menu-item.ui")]
pub struct MenuItemPriv {
    #[property(get, set)]
    pub item_id: RefCell<String>,
    #[property(get, set)]
    pub id: RefCell<i32>,
    #[property(get, set)]
    pub theme_path: RefCell<Option<String>>,
    #[template_child]
    pub outer_container: TemplateChild<gtk::CenterBox>,
    #[template_child]
    pub checkbox: TemplateChild<gtk::CheckButton>,
    #[template_child]
    pub icon: TemplateChild<gtk::Image>,
    #[template_child]
    pub label: TemplateChild<gtk::Label>,
    #[template_child]
    pub arrow: TemplateChild<gtk::Button>,
    pub action_tx: RefCell<Option<UnboundedSender<(String, MenuItemAction)>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for MenuItemPriv {
    const NAME: &'static str = "SystrayMenuItemWidget";
    type Type = MenuItem;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        klass.set_css_name("menu-item");
        // klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[derived_properties]
impl ObjectImpl for MenuItemPriv {
    fn constructed(&self) {
        self.parent_constructed();
        let container_gest = GestureClick::new();
        container_gest.set_button(gdk::BUTTON_PRIMARY);
        container_gest.connect_released(Self::container_clicked);
        self.outer_container.borrow().add_controller(container_gest);
        let hover_gest = EventControllerMotion::new();
        hover_gest.connect_enter(Self::container_hover);
        self.outer_container.borrow().add_controller(hover_gest);
        let this = self.obj().clone();
        self.arrow.get().connect_clicked(move |_| {
            let action_tx = this.imp().action_tx.borrow().clone().unwrap();
            action_tx
                .send((this.item_id(), MenuItemAction::OpenMenu(this.id())))
                .unwrap();
        });
    }
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}
impl MenuItemPriv {
    fn container_clicked(gest: &GestureClick, _n: i32, _x: f64, _y: f64) {
        let container = gest.widget();
        let item = container.parent().unwrap().downcast::<MenuItem>().unwrap();
        let action_tx = item.imp().action_tx.borrow_mut().clone().unwrap();
        action_tx
            .send((item.item_id(), MenuItemAction::Clicked(item.id())))
            .unwrap();
        if item.imp().arrow.borrow().is_visible() {
            action_tx
                .send((item.item_id(), MenuItemAction::OpenMenu(item.id())))
                .unwrap();
        }
    }
    fn container_hover(gest: &EventControllerMotion, _x: f64, _y: f64) {
        let container = gest.widget();
        let item = container.parent().unwrap().downcast::<MenuItem>().unwrap();
        let action_tx = item.imp().action_tx.borrow_mut().clone().unwrap();
        action_tx
            .send((item.item_id(), MenuItemAction::Hovered(item.id())))
            .unwrap();
    }
}

impl WidgetImpl for MenuItemPriv {}

impl MenuItem {
    pub fn new(action_tx: UnboundedSender<(String, MenuItemAction)>) -> Self {
        let this: Self = Object::builder().build();
        this.imp().action_tx.replace(Some(action_tx));

        this
    }
    pub fn set_child(&self, child: &impl IsA<gtk::Widget>) {
        child.set_parent(self.upcast_ref::<gtk::Widget>());
    }
    pub fn update_from_layout_child(&self, layout_child: &LayoutChild) {
        if let Some(LayoutProperty::Visible(false)) =
            layout_child.properties.get(layout::LAYOUT_PROP_VISIBLE)
        {
            self.set_visible(false);
            return;
        } else {
            self.set_visible(true);
        }
        if let Some(LayoutProperty::Type(item_type)) = layout_child
            .properties
            .get(crate::status_notifier::layout::LAYOUT_PROP_TYPE)
        {
            match item_type {
                TypeProperty::Standard => {}
                _ => {
                    self.set_visible(false);
                    return;
                }
            }
        }
        self.imp().id.replace(layout_child.id);
        let enabled = if let Some(LayoutProperty::Enabled(false)) =
            layout_child.properties.get(layout::LAYOUT_PROP_ENABLED)
        {
            false
        } else {
            true
        };
        self.set_sensitive(enabled);
        if let LayoutProperty::ToggleType(toggle_type) = layout_child
            .properties
            .get(layout::LAYOUT_PROP_TOGGLE_TYPE)
            .unwrap_or(&LayoutProperty::ToggleType(ToggleProperty::None))
        {
            let toggle_state = if let Some(LayoutProperty::ToggleState(true)) = layout_child
                .properties
                .get(layout::LAYOUT_PROP_TOGGLE_STATE)
            {
                true
            } else {
                false
            };
            let checkbox = self.imp().checkbox.clone();
            match toggle_type {
                ToggleProperty::Radio => {
                    checkbox.set_visible(true);
                    // TODO it will probably not work
                    checkbox.set_group(Some(&checkbox));
                    checkbox.set_active(toggle_state);
                }
                ToggleProperty::Checkmark => {
                    checkbox.set_visible(true);
                    checkbox.set_group(None::<&gtk::CheckButton>);
                    checkbox.set_active(toggle_state);
                }
                ToggleProperty::None => {
                    checkbox.set_visible(false);
                }
            }
        } else {
            self.imp().checkbox.clone().set_visible(false);
        }
        if let Some(LayoutProperty::IconName(icon_name)) =
            layout_child.properties.get(layout::LAYOUT_PROP_ICON_NAME)
        {
            let paintable =
                icon::icon_from_name(icon_name, self.imp().theme_path.borrow().as_deref(), 25, 1)
                    .ok();
            self.imp().icon.borrow().set_paintable(paintable.as_ref());
            self.imp().icon.borrow().set_visible(true);
        } else if let Some(LayoutProperty::IconData(paintable)) =
            layout_child.properties.get(layout::LAYOUT_PROP_ICON_DATA)
        {
            self.imp().icon.borrow().set_paintable(Some(paintable));
            self.imp().icon.borrow().set_visible(true);
        } else {
            self.imp().icon.borrow().set_paintable(None::<&Paintable>);
            self.imp().icon.borrow().set_visible(false);
        }
        if let Some(LayoutProperty::Label(label)) =
            layout_child.properties.get(layout::LAYOUT_PROP_LABEL)
        {
            let label_strip_underscore = strip_underscore_from_label(label);
            self.imp().label.clone().set_text(&label_strip_underscore);
        }
        let arrow = self.imp().arrow.clone();
        if let Some(LayoutProperty::ChildrenDisplay(true)) = layout_child
            .properties
            .get(layout::LAYOUT_PROP_CHILDREN_DISPLAY)
        {
            arrow.set_visible(true);
        } else {
            arrow.set_visible(false);
        }
    }
}

fn strip_underscore_from_label(label: &str) -> String {
    let mut label_strip_underscore = String::new();
    let chars: Vec<char> = label.chars().collect();
    for (i, char) in chars.iter().enumerate() {
        let previous = i
            .checked_sub(1)
            .and_then(|idx| chars.get(idx))
            .unwrap_or(&' ');
        if char == &'_' && previous != &'_' {
            continue;
        }
        label_strip_underscore.push(*char);
    }
    label_strip_underscore
}
