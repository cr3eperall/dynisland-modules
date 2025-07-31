use std::cell::RefCell;

use dynisland_core::{
    abi::{
        glib,
        gtk::{self, EventControllerScroll, EventControllerScrollFlags},
    },
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
};
use glib::{
    prelude::ObjectExt,
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
        InitializingObject,
    },
    types::StaticTypeExt,
    Object, Properties,
};
use gtk::{
    prelude::WidgetExt,
    subclass::widget::{
        CompositeTemplateClass, CompositeTemplateDisposeExt, CompositeTemplateInitializingExt,
        WidgetClassExt, WidgetImpl,
    },
    BinLayout, CompositeTemplate, TemplateChild,
};

use super::{battery::Battery, graph::Graph};
use crate::upower::device::HistoryEntry;

glib::wrapper! {
    pub struct Expanded(ObjectSubclass<ExpandedPriv>)
    @extends gtk::Widget;
}

#[derive(CompositeTemplate, Properties)]
#[properties(wrapper_type = Expanded)]
#[template(resource = "/com/github/cr3eperall/dynislandModules/powerModule/expanded.ui")]
pub struct ExpandedPriv {
    #[template_child]
    pub graph: TemplateChild<Graph>,
    #[property(get, set)]
    pub low_battery_color: RefCell<String>,
    #[property(get, set)]
    pub charging_color: RefCell<String>,
    #[property(get, set)]
    pub normal_color: RefCell<String>,
    //TODO add low battery treshold
}

impl Default for ExpandedPriv {
    fn default() -> Self {
        Self {
            graph: TemplateChild::default(),
            low_battery_color: RefCell::new("red".to_string()),
            charging_color: RefCell::new("green".to_string()),
            normal_color: RefCell::new("white".to_string()),
        }
    }
}

#[glib::object_subclass]
impl ObjectSubclass for ExpandedPriv {
    const NAME: &'static str = "PowerExpandedWidget";
    type Type = Expanded;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        // if you use custom widgets from core you need to ensure the type
        Battery::ensure_type();
        klass.set_layout_manager_type::<BinLayout>();
        klass.bind_template();
        // Warning: template callbacks only work if the module is embedded
        // so don't call `klass.bind_template_instance_callbacks();` or dynisland will crash
        // manually connect signals in `ObjectImpl::constructed` instead
    }

    fn instance_init(obj: &InitializingObject<Self>) {
        obj.init_template();
    }
}

#[glib::derived_properties]
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
    /// registered properties:
    /// * `points`: `Vec<(u64, f64)>`
    pub fn new(activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        let contr = EventControllerScroll::new(
            EventControllerScrollFlags::VERTICAL.union(EventControllerScrollFlags::HORIZONTAL),
        );
        let gr = this.imp().graph.clone();
        contr.connect_scroll(move |_ev, _x, y| {
            // log::debug!("scrolling, {:?}, x:{x}, y:{y}", ev.current_event_state());
            gr.set_max_duration_secs((gr.max_duration_secs() as f64 + (y * 80.0)) as u32);
            gr.queue_draw();
            glib::Propagation::Proceed
        });
        this.add_controller(contr);

        // register the property if it doesn't exist
        // this way we can update multiple widgets with the same property
        let _ = activity.add_dynamic_property("points", Vec::<HistoryEntry>::new());

        let minimal = this.clone();
        activity
            .subscribe_to_property("points", move |new_value| {
                let points = cast_dyn_any!(new_value, Vec::<HistoryEntry>).unwrap();
                minimal.imp().graph.set_points(points);
            })
            .unwrap();

        // let _ = activity.add_dynamic_property("charging", false);

        // let minimal = this.clone();
        // activity
        //     .subscribe_to_property("charging", move |new_value| {
        //         let charging = *cast_dyn_any!(new_value, bool).unwrap();
        //         minimal.imp().battery.set_charging(charging);
        //         if charging {
        //             minimal
        //                 .imp()
        //                 .battery
        //                 .set_fill_color(minimal.imp().charging_color.borrow().clone());
        //         }
        //     })
        //     .unwrap();
        this
    }
}
