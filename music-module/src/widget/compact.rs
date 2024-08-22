use dynisland_core::graphics::widgets::scrolling_label::ScrollingLabel;

use crate::module::MusicConfig;

use super::visualizer::Visualizer;

use glib::{
    subclass::{
        object::{ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassIsExt},
        InitializingObject,
    },
    Object,
};
use gtk::{
    prelude::*,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateInitializingExt, WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

glib::wrapper! {
    pub struct Compact(ObjectSubclass<CompactPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Default)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/musicModule/compact.ui")]
pub struct CompactPriv {
    #[template_child]
    pub image: TemplateChild<gtk::Image>,
    #[template_child]
    pub song_name: TemplateChild<ScrollingLabel>,
}

#[glib::object_subclass]
impl ObjectSubclass for CompactPriv {
    const NAME: &'static str = "MusicCompactWidget";
    type Type = Compact;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        ScrollingLabel::ensure_type();
        Visualizer::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

impl ObjectImpl for CompactPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }
}

impl WidgetImpl for CompactPriv {}

impl Compact {
    pub fn new(config: &MusicConfig) -> Self {
        let this: Self = Object::builder().build();
        this.imp()
            .image
            .set_file(Some(&config.default_album_art_url));

        this
    }
}
