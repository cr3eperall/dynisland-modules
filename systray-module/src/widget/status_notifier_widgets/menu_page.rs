use std::{borrow::Borrow, cell::RefCell};

use dynisland_core::abi::{
    gdk::subclass::prelude::*,
    glib::{self, derived_properties, Properties},
    gtk,
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
use tokio::sync::mpsc::UnboundedSender;

use super::menu_item::{MenuItem, MenuItemAction};
use crate::{
    config::MenuHeightMode,
    status_notifier::layout::{self, LayoutChild, LayoutProperty, TypeProperty},
};

glib::wrapper! {
    pub struct MenuPage(ObjectSubclass<MenuPagePriv>)
    @extends gtk::Widget;
}

pub const MAX_CACHED_ITEMS: usize = 5;

#[derive(CompositeTemplate, Default, Properties)]
#[properties(wrapper_type = MenuPage)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/systrayModule/menu-page.ui")]
pub struct MenuPagePriv {
    #[property(get, set)]
    pub item_id: RefCell<String>,
    #[property(get, set)]
    pub id: RefCell<i32>,
    #[template_child]
    pub scrolled_window: TemplateChild<gtk::ScrolledWindow>,
    #[template_child]
    pub revealer: TemplateChild<gtk::Revealer>,
    #[template_child]
    pub back_button: TemplateChild<gtk::Button>,
    #[template_child]
    pub menu_container: TemplateChild<gtk::Box>,
    pub unused_menu_items: RefCell<Vec<MenuItem>>,
    pub action_tx: RefCell<Option<UnboundedSender<(String, MenuItemAction)>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for MenuPagePriv {
    const NAME: &'static str = "SystrayMenuPageWidget";
    type Type = MenuPage;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        MenuItem::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        klass.set_css_name("menu-page");
        // If you use template callbacks (for example running a function when a button is pressed), uncomment this
        // klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[derived_properties]
impl ObjectImpl for MenuPagePriv {
    fn constructed(&self) {
        self.parent_constructed();
        let scrolled_window = self.scrolled_window.get();
        self.revealer.connect_child_revealed_notify(move |rev| {
            let reveal = rev.is_child_revealed();
            scrolled_window.set_propagate_natural_height(reveal);
        });
        let this = self.obj().clone();
        self.back_button.connect_clicked(move |_| {
            let action_tx = this.imp().action_tx.borrow().clone().unwrap();
            action_tx
                .send((this.item_id(), MenuItemAction::GoBack))
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

impl WidgetImpl for MenuPagePriv {}

impl MenuPage {
    pub fn new(action_tx: UnboundedSender<(String, MenuItemAction)>) -> Self {
        let this: Self = Object::builder().build();
        this.imp().action_tx.replace(Some(action_tx));
        this
    }
    pub fn revealer(&self) -> gtk::Revealer {
        self.imp().revealer.borrow().get()
    }
    pub fn set_reveal(&self, reveal: bool, animation_right: bool, height_mode: MenuHeightMode) {
        match animation_right {
            true => self
                .revealer()
                .set_transition_type(gtk::RevealerTransitionType::SlideRight),
            false => self
                .revealer()
                .set_transition_type(gtk::RevealerTransitionType::SlideLeft),
        }
        self.revealer().set_reveal_child(reveal);
        match height_mode {
            MenuHeightMode::Max => {
                self.imp()
                    .scrolled_window
                    .get()
                    .set_propagate_natural_height(true);
            }
            MenuHeightMode::Current => {
                self.imp()
                    .scrolled_window
                    .set_propagate_natural_height(reveal);
            }
            MenuHeightMode::TwoStep => {
                if reveal {
                    self.imp()
                        .scrolled_window
                        .get()
                        .set_propagate_natural_height(true);
                }
            }
        }
    }
    pub fn update_from_layout_parent(&self, layout_parent: &LayoutChild, item_id: String) {
        match layout_parent
            .properties
            .get(layout::LAYOUT_PROP_CHILDREN_DISPLAY)
        {
            Some(LayoutProperty::ChildrenDisplay(true)) => {}
            _ => return,
        }
        self.set_id(layout_parent.id);
        self.set_item_id(item_id);
        self.imp().back_button.set_visible(layout_parent.id != 0);
        let menu_container = self.imp().menu_container.clone();

        let mut old_items = self.imp().unused_menu_items.borrow_mut();
        let (mut available_items, mut available_separators) =
            extract_existing_widgets(&menu_container);
        while let Some(item) = old_items.pop() {
            available_items.push(item);
        }

        for layout_child in layout_parent.children.iter() {
            match layout_child.properties.get(layout::LAYOUT_PROP_TYPE) {
                Some(LayoutProperty::Type(TypeProperty::Separator)) => {
                    let separator = available_separators.pop().unwrap_or(
                        gtk::Separator::builder()
                            .orientation(gtk::Orientation::Horizontal)
                            .build(),
                    );
                    menu_container.append(&separator);
                }
                _ => {
                    let menu_item = available_items.pop().unwrap_or(MenuItem::new(
                        self.imp().action_tx.borrow().clone().unwrap(),
                    ));
                    menu_item.update_from_layout_child(layout_child);
                    menu_item.set_item_id(self.item_id());
                    menu_container.append(&menu_item);
                }
            }
        }
        for (i, item) in available_items.into_iter().enumerate() {
            if i == MAX_CACHED_ITEMS {
                break;
            }
            old_items.push(item);
        }
    }
}

fn extract_existing_widgets(menu_container: &gtk::Box) -> (Vec<MenuItem>, Vec<gtk::Separator>) {
    let mut all_children = Vec::new();
    let mut available_items = Vec::new();
    let mut separators = Vec::new();

    for child in menu_container
        .observe_children()
        .iter::<glib::Object>()
        .flatten()
    {
        all_children.push(child.clone().downcast::<gtk::Widget>().unwrap());
        match child.downcast::<MenuItem>() {
            Ok(child) => available_items.push(child),
            Err(obj) => match obj.downcast::<gtk::Separator>() {
                Ok(child) => separators.push(child),
                Err(_) => {}
            },
        };
    }
    for child in all_children {
        menu_container.remove(&child);
    }
    (available_items, separators)
}
