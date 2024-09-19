use std::{cell::RefCell, sync::Arc, time::Duration};

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
    BinLayout, CompositeTemplate, TemplateChild,
};
use tokio::sync::{
    mpsc::{UnboundedReceiver, UnboundedSender},
    Mutex,
};

use super::{visualizer::Visualizer, UIAction};

glib::wrapper! {
    pub struct Expanded(ObjectSubclass<ExpandedPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate)]
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

    pub action_tx: RefCell<UnboundedSender<UIAction>>,
    pub action_rx: Arc<Mutex<UnboundedReceiver<UIAction>>>,
}

impl Default for ExpandedPriv {
    fn default() -> Self {
        let (action_tx, action_rx) = tokio::sync::mpsc::unbounded_channel();
        ExpandedPriv {
            action_tx: RefCell::new(action_tx),
            action_rx: Arc::new(Mutex::new(action_rx)),
            image: Default::default(),
            song_name: Default::default(),
            artist_name: Default::default(),
            elapsed_time: Default::default(),
            progress_bar: Default::default(),
            remaining_time: Default::default(),
            shuffle: Default::default(),
            previous: Default::default(),
            play_pause: Default::default(),
            next: Default::default(),
            repeat: Default::default(),
        }
    }
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
        // Warning: template callbacks only work if the module is embedded
        // If the module is dynamically loaded it works if only one module uses template callbacks
        klass.bind_template_instance_callbacks();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

// Warning: template callbacks only work if the module is embedded
// If the module is dynamically loaded it works if only one module uses template callbacks
#[gtk::template_callbacks]
impl Expanded {
    #[template_callback]
    fn handle_shuffle(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .send(UIAction::Shuffle)
            .unwrap();
    }
    #[template_callback]
    fn handle_previous(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .send(UIAction::Previous)
            .unwrap();
    }
    #[template_callback]
    fn handle_play_pause(&self, _button: &gtk::Button) {
        self.imp()
            .action_tx
            .borrow()
            .send(UIAction::PlayPause)
            .unwrap();
    }
    #[template_callback]
    fn handle_next(&self, _button: &gtk::Button) {
        self.imp().action_tx.borrow().send(UIAction::Next).unwrap();
    }
    #[template_callback]
    fn handle_loop(&self, _button: &gtk::Button) {
        self.imp().action_tx.borrow().send(UIAction::Loop).unwrap();
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
    pub fn new() -> Self {
        let this: Self = Object::builder().build();
        let imp = this.imp();
        imp.progress_bar.set_range(0.0, 1.0);
        imp.progress_bar.set_increments(0.0035, 0.1);

        let action_tx = imp.action_tx.borrow().clone();
        // let release_gesture = GestureClick::new();
        // release_gesture.set_button(gdk::BUTTON_PRIMARY);
        // release_gesture.connect_unpaired_release(move |gest, _, _, _, _| {
        //     let prog = gest.widget().downcast::<gtk::Scale>().unwrap();
        //     dynisland_core::abi::log::debug!("unpaired release");
        //     action_tx
        //         .send(UIAction::SetPosition(Duration::from_millis(
        //             prog.value() as u64
        //         )))
        //         .expect("failed to send seek message");
        // });
        // imp.progress_bar.add_controller(release_gesture);

        let gest = gtk::EventControllerLegacy::new();
        gest.connect_event(move |gest, event| {
            let cont = event
                .modifier_state()
                .intersects(gdk::ModifierType::BUTTON1_MASK);
            if event.event_type() == gdk::EventType::ButtonRelease {
                let prog = gest.widget().downcast::<gtk::Scale>().unwrap();
                if cont {
                    action_tx
                        .send(UIAction::SetPosition(Duration::from_millis(
                            prog.value() as u64
                        )))
                        .expect("failed to send seek message");
                }
            }
            glib::Propagation::Proceed
        });
        imp.progress_bar.add_controller(gest);

        this
    }
}
