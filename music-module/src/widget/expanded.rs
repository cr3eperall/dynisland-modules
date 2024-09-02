use std::{cell::RefCell, time::Duration};

use dynisland_core::{
    abi::{gdk, glib, gtk},
    graphics::widgets::scrolling_label::ScrollingLabel,
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
    prelude::*,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        CompositeTemplateInstanceCallbacksClass, WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, GestureClick, TemplateChild,
};
use tokio::sync::mpsc::UnboundedSender;

use super::{visualizer::Visualizer, UIAction};
use crate::config::MusicConfig;

glib::wrapper! {
    pub struct Expanded(ObjectSubclass<ExpandedPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/musicModule/expanded.ui")]
pub struct ExpandedPriv {
    #[template_child]
    pub image: TemplateChild<gtk::Image>,
    #[template_child]
    pub song_name: TemplateChild<ScrollingLabel>,
    #[template_child]
    pub artist_name: TemplateChild<gtk::Label>,

    #[template_child]
    pub elapsed_time: TemplateChild<gtk::Label>,
    #[template_child]
    pub progress_bar: TemplateChild<gtk::Scale>,
    #[template_child]
    pub remaining_time: TemplateChild<gtk::Label>,

    #[template_child]
    pub shuffle: TemplateChild<gtk::Button>,
    #[template_child]
    pub previous: TemplateChild<gtk::Button>,
    #[template_child]
    pub play_pause: TemplateChild<gtk::Button>,
    #[template_child]
    pub next: TemplateChild<gtk::Button>,
    #[template_child]
    pub repeat: TemplateChild<gtk::Button>,

    pub action_tx: RefCell<Option<UnboundedSender<UIAction>>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ExpandedPriv {
    const NAME: &'static str = "MusicExpandedWidget";
    type Type = Expanded;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        ScrollingLabel::ensure_type();
        Visualizer::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}
#[gtk::template_callbacks]
impl Expanded {
    #[template_callback]
    fn handle_shuffle(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .as_ref()
            .unwrap()
            .send(UIAction::Shuffle)
            .unwrap();
    }
    #[template_callback]
    fn handle_previous(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .as_ref()
            .unwrap()
            .send(UIAction::Previous)
            .unwrap();
    }
    #[template_callback]
    fn handle_play_pause(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .as_ref()
            .unwrap()
            .send(UIAction::PlayPause)
            .unwrap();
    }
    #[template_callback]
    fn handle_next(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .as_ref()
            .unwrap()
            .send(UIAction::Next)
            .unwrap();
    }
    #[template_callback]
    fn handle_loop(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .as_ref()
            .unwrap()
            .send(UIAction::Loop)
            .unwrap();
    }
}

impl ObjectImpl for ExpandedPriv {
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

impl WidgetImpl for ExpandedPriv {}

impl Expanded {
    pub fn new(config: &MusicConfig, action_tx: UnboundedSender<UIAction>) -> Self {
        let this: Self = Object::builder().build();
        let imp = this.imp();
        imp.action_tx.replace(Some(action_tx.clone()));
        imp.image.set_file(Some(&config.default_album_art_url));
        imp.progress_bar.set_range(0.0, 1.0);
        imp.progress_bar.set_increments(0.0035, 0.1);

        let release_gesture = GestureClick::new();
        release_gesture.set_button(gdk::BUTTON_PRIMARY);
        release_gesture.connect_unpaired_release(move |gest, _, _, _, _| {
            let prog = gest.widget().downcast::<gtk::Scale>().unwrap();
            action_tx
                .send(UIAction::SetPosition(Duration::from_millis(
                    prog.value() as u64
                )))
                .expect("failed to send seek message");
        });
        imp.progress_bar.add_controller(release_gesture);

        this
    }
}
