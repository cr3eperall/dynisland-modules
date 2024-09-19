use std::{cell::RefCell, sync::Arc};

use dynisland_core::{
    abi::{glib, gtk},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
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

use super::status_notifier_widgets::{menu_item::MenuItemAction, menu_page::MenuPage};
use crate::{
    config::MenuHeightMode,
    status_notifier::layout::{Layout, LayoutChild},
};

glib::wrapper! {
    pub struct Expanded(ObjectSubclass<ExpandedPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/systrayModule/expanded.ui")]
pub struct ExpandedPriv {
    pub heigth_mode: RefCell<MenuHeightMode>,
    pub cleanup_tx: RefCell<Option<tokio::sync::broadcast::Sender<()>>>,
    pub item_id: RefCell<String>,
    pub layout: RefCell<Layout>,
    pub current_path: RefCell<Vec<usize>>,
    #[template_child]
    pub container: TemplateChild<gtk::Box>,
    pub action_tx: RefCell<UnboundedSender<(String, MenuItemAction)>>,
    pub action_rx: Arc<Mutex<UnboundedReceiver<(String, MenuItemAction)>>>,
    inner_action_tx: RefCell<UnboundedSender<(String, MenuItemAction)>>,
    inner_action_rx: RefCell<Option<UnboundedReceiver<(String, MenuItemAction)>>>,
}

impl Default for ExpandedPriv {
    fn default() -> Self {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let (outer_tx, outer_rx) = tokio::sync::mpsc::unbounded_channel();
        Self {
            heigth_mode: RefCell::new(MenuHeightMode::default()),
            cleanup_tx: RefCell::new(None),
            item_id: RefCell::new(String::new()),
            layout: RefCell::new(Layout::default()),
            current_path: RefCell::new(Vec::new()),
            container: TemplateChild::default(),
            action_tx: RefCell::new(outer_tx),
            action_rx: Arc::new(Mutex::new(outer_rx)),
            inner_action_tx: RefCell::new(tx),
            inner_action_rx: RefCell::new(Some(rx)),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ExpandedPriv {
    const NAME: &'static str = "SystrayExpandedWidget";
    type Type = Expanded;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        MenuPage::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        // If you use template callbacks (for example running a function when a button is pressed), uncomment this
        // klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for ExpandedPriv {
    fn constructed(&self) {
        self.parent_constructed();
        let container = self.container.get();
        let action_tx = self.inner_action_tx.borrow().clone();
        let menu_page1 = MenuPage::new(action_tx.clone());
        menu_page1.set_reveal(false, true, *self.heigth_mode.borrow());
        container.append(&menu_page1);
        let menu_page2 = MenuPage::new(action_tx.clone());
        menu_page2.set_reveal(true, false, *self.heigth_mode.borrow());
        container.append(&menu_page2);
        let menu_page3 = MenuPage::new(action_tx);
        menu_page3.set_reveal(false, false, *self.heigth_mode.borrow());
        container.append(&menu_page3);
    }
    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
        self.dispose_template();
    }
}

impl WidgetImpl for ExpandedPriv {}

impl Expanded {
    /// registered properties:
    /// * `height-mode`: `MenuHeightMode`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        glib::MainContext::default().spawn_local({
            let this = this.clone();
            let mut inner_action_rx = this.imp().inner_action_rx.borrow_mut().take().unwrap();
            async move {
                while let Some(action) = inner_action_rx.recv().await {
                    match action {
                        (_, MenuItemAction::Clicked(_)) | (_, MenuItemAction::Hovered(_)) => {
                            this.imp()
                                .action_tx
                                .borrow_mut()
                                .clone()
                                .send(action)
                                .unwrap();
                        }
                        (_, MenuItemAction::GoBack) => {
                            this.go_back();
                        }
                        (_, MenuItemAction::OpenMenu(id)) => {
                            this.imp()
                                .action_tx
                                .borrow_mut()
                                .clone()
                                .send(action)
                                .unwrap();
                            this.open_menu(id);
                        }
                    }
                }
            }
        });
        let _ = activity.add_dynamic_property("height-mode", MenuHeightMode::default());

        let expanded = this.clone();
        activity
            .subscribe_to_property("height-mode", move |new_value| {
                let value_mode = cast_dyn_any!(new_value, MenuHeightMode).unwrap();
                expanded.imp().heigth_mode.replace(*value_mode);
            })
            .unwrap();
        this
    }
    pub fn go_back(&self) {
        let layout = self.imp().layout.borrow();
        let mut current_path = self.imp().current_path.borrow_mut();

        let prev_page = self.imp().container.get().first_child().unwrap();
        let page = prev_page.next_sibling().unwrap();
        let next_page = self.imp().container.get().last_child().unwrap();

        let prev_page = prev_page.downcast::<MenuPage>().unwrap();
        let page = page.downcast::<MenuPage>().unwrap();
        let next_page = next_page.downcast::<MenuPage>().unwrap();

        let mut next_path = current_path.clone();
        next_path.pop();
        let (parent, child) = find_child_from_path(&layout, &next_path);
        if child.id != prev_page.id() {
            return;
        }
        next_page.update_from_layout_parent(parent, self.imp().item_id.borrow().clone());
        current_path.pop();

        prev_page.set_reveal(true, true, *self.imp().heigth_mode.borrow());
        page.set_reveal(false, false, *self.imp().heigth_mode.borrow());
        next_page.set_reveal(false, false, *self.imp().heigth_mode.borrow());

        self.imp()
            .container
            .get()
            .reorder_child_after(&next_page, None::<&gtk::Widget>);
    }
    pub fn open_menu(&self, id: i32) {
        let layout = self.imp().layout.borrow();
        let mut current_path = self.imp().current_path.borrow_mut();

        let prev_page = self.imp().container.get().first_child().unwrap();
        let page = prev_page.next_sibling().unwrap();
        let next_page = self.imp().container.get().last_child().unwrap();

        let prev_page = prev_page.downcast::<MenuPage>().unwrap();
        let page = page.downcast::<MenuPage>().unwrap();
        let next_page = next_page.downcast::<MenuPage>().unwrap();

        let mut next_path = current_path.clone();
        next_path.push(id as usize);
        let (parent, child) = find_child_from_path(&layout, &next_path);
        if parent.id != page.id() {
            return;
        }
        next_page.update_from_layout_parent(child, self.imp().item_id.borrow().clone());
        current_path.push(id as usize);

        prev_page.set_reveal(false, false, *self.imp().heigth_mode.borrow());
        page.set_reveal(false, true, *self.imp().heigth_mode.borrow());
        next_page.set_reveal(true, false, *self.imp().heigth_mode.borrow());

        self.imp()
            .container
            .get()
            .reorder_child_after(&prev_page, Some(&next_page));
    }
    pub fn set_layout(&self, layout: Layout, path: Option<Vec<usize>>, item_id: String) {
        self.imp().layout.replace(layout);
        self.imp().item_id.replace(item_id);
        if let Some(path) = path {
            self.imp().current_path.replace(path);
        }
        let layout = self.imp().layout.borrow();
        let current_path = self.imp().current_path.borrow();

        let prev_page = self.imp().container.get().first_child().unwrap();
        let page = prev_page.next_sibling().unwrap();
        let next_page = self.imp().container.get().last_child().unwrap();

        let prev_page = prev_page.downcast::<MenuPage>().unwrap();
        let page = page.downcast::<MenuPage>().unwrap();
        let next_page = next_page.downcast::<MenuPage>().unwrap();

        prev_page.revealer().set_reveal_child(false);
        page.revealer().set_reveal_child(true);
        next_page.revealer().set_reveal_child(false);
        if current_path.is_empty() {
            page.update_from_layout_parent(&layout.root, self.imp().item_id.borrow().clone());
        } else if current_path.len() == 1 {
            prev_page.update_from_layout_parent(&layout.root, self.imp().item_id.borrow().clone());
            let child = &layout.root.children[current_path[0]];
            page.update_from_layout_parent(child, self.imp().item_id.borrow().clone());
        } else {
            let (parent, child) = find_child_from_path(&layout, &current_path);
            prev_page.update_from_layout_parent(parent, self.imp().item_id.borrow().clone());
            page.update_from_layout_parent(child, self.imp().item_id.borrow().clone());
        }
    }
}

fn find_child_from_path<'a>(
    layout: &'a Layout,
    current_path: &'_ Vec<usize>,
) -> (&'a LayoutChild, &'a LayoutChild) {
    let mut parent = &layout.root;
    let mut child = &layout.root;
    let mut path_idx = 0;
    while child.id as usize != *current_path.last().unwrap_or(&((child.id + 1) as usize)) {
        if let Some(next_child_id) = current_path.get(path_idx) {
            let next_child = child
                .children
                .iter()
                .find(|c| c.id == *next_child_id as i32);
            match next_child {
                Some(next_child) => {
                    parent = child;
                    child = next_child;
                }
                None => {
                    break;
                }
            }
        } else {
            break;
        }
        path_idx += 1;
    }
    (parent, child)
}
