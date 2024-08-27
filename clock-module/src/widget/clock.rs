use std::{cell::RefCell, f64::consts::PI};

use chrono::{Local, Timelike};
use dynisland_core::{
    abi::{gdk, glib, gtk, log},
    cast_dyn_any,
    dynamic_activity::DynamicActivity,
};
use gdk::RGBA;
use glib::{
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt, ObjectSubclassIsExt},
    },
    Object, Properties,
};
use gtk::{
    graphene::Rect,
    prelude::*,
    subclass::widget::{WidgetClassExt, WidgetImpl},
    BinLayout,
};

glib::wrapper! {
    pub struct Clock(ObjectSubclass<ClockPriv>)
    @extends gtk::Widget;
}
#[derive(Properties)]
#[properties(wrapper_type = Clock)]
pub struct ClockPriv {
    #[property(get, set, type = String)]
    hour_hand_color: RefCell<RGBA>,
    #[property(get, set, type = String)]
    minute_hand_color: RefCell<RGBA>,
    #[property(get, set, type = String)]
    tick_color: RefCell<RGBA>,
    #[property(get, set, type = String)]
    circle_color: RefCell<RGBA>,

    time: RefCell<chrono::DateTime<Local>>,
}

#[glib::object_subclass]
impl ObjectSubclass for ClockPriv {
    const NAME: &'static str = "ClockMinimalWidget";
    type Type = Clock;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
    }
}
#[allow(clippy::derivable_impls)]
impl Default for ClockPriv {
    fn default() -> Self {
        Self {
            hour_hand_color: RefCell::new(RGBA::parse("white").unwrap()),
            minute_hand_color: RefCell::new(RGBA::parse("white").unwrap()),
            circle_color: RefCell::new(RGBA::parse("lightgray").unwrap()),
            tick_color: RefCell::new(RGBA::parse("lightgray").unwrap()),
            time: RefCell::new(Local::now()),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for ClockPriv {
    fn constructed(&self) {
        self.parent_constructed();
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "hour-hand-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.hour_hand_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid hour hand color: {name}");
                }
            }
            "minute-hand-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.minute_hand_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid minute hand color: {name}");
                }
            }
            "tick-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.tick_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid tick color: {name}");
                }
            }
            "circle-color" => {
                let name: String = value.get().unwrap();
                if let Ok(color) = RGBA::parse(&name) {
                    self.circle_color.replace(color);
                    self.obj().queue_draw();
                } else {
                    log::warn!("invalid circle color: {name}");
                }
            }
            _ => {
                log::warn!("Clock: invalid property received: {}", pspec.name());
            }
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "hour-hand-color" => self.hour_hand_color.borrow().to_string().to_value(),
            "minute-hand-color" => self.minute_hand_color.borrow().to_string().to_value(),
            "tick-color" => self.tick_color.borrow().to_string().to_value(),
            "circle-color" => self.circle_color.borrow().to_string().to_value(),
            _ => self.derived_property(id, pspec),
        }
    }
}

impl WidgetImpl for ClockPriv {
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (self.obj().width() as f32, self.obj().height() as f32);
        let size = w.min(h);
        let (x, y) = ((w - size) / 2.0, (h - size) / 2.0);
        let rect = Rect::new(x, y, w, h);

        let size = size as f64;
        let ctx = snapshot.append_cairo(&rect);

        //circle
        ctx.set_source_color(&self.circle_color.borrow());
        ctx.set_line_width(size * 0.04);
        ctx.arc(size / 2.0, size / 2.0, size * 0.48, 0.0, PI * 2.0);
        ctx.stroke().unwrap();

        //ticks
        ctx.set_source_color(&self.tick_color.borrow());
        ctx.save().unwrap();
        ctx.translate(size / 2.0, size / 2.0);
        ctx.save().unwrap();
        ctx.set_line_width(size * 0.04);
        for _ in 0..12 {
            ctx.rotate(PI * 2.0 / 12.0);
            ctx.move_to(0.0, size * 0.44);
            ctx.line_to(0.0, size * 0.35);
            ctx.stroke().unwrap();
        }
        ctx.restore().unwrap();

        let hour = self.time.borrow().hour() as f64;
        let minute = self.time.borrow().minute() as f64;

        //hour hand
        ctx.set_line_cap(gdk::cairo::LineCap::Round);
        ctx.save().unwrap();
        ctx.rotate(2.0 * PI * (hour / 12.0));
        ctx.set_line_width(size * 0.06);
        ctx.set_source_color(&self.hour_hand_color.borrow());
        ctx.move_to(0.0, 0.0);
        ctx.line_to(0.0, -size * 0.27);
        ctx.stroke().unwrap();
        ctx.restore().unwrap();

        //minute hand
        ctx.save().unwrap();
        ctx.rotate(2.0 * PI * (minute / 60.0));
        ctx.set_line_width(size * 0.05);
        ctx.set_source_color(&self.minute_hand_color.borrow());
        ctx.move_to(0.0, 0.0);
        ctx.line_to(0.0, -size * 0.42);
        ctx.stroke().unwrap();
        ctx.restore().unwrap();
    }
}

#[allow(clippy::new_without_default)]
impl Clock {
    /// registered properties:
    pub fn new(dynamic_activity: &mut DynamicActivity) -> Self {
        let this: Self = Object::builder().build();
        this.add_css_class("clock");
        let clock = this.clone();
        if dynamic_activity.get_property_any("time").is_err() {
            dynamic_activity
                .add_dynamic_property("time", Local::now())
                .unwrap();
        }
        dynamic_activity
            .subscribe_to_property("time", move |value| {
                let time = cast_dyn_any!(value, chrono::DateTime<chrono::Local>).unwrap();
                clock.set_time(*time);
            })
            .unwrap();

        this
    }

    /// must be used from the gtk main context
    pub fn get_time(&self) -> chrono::DateTime<Local> {
        *self.imp().time.borrow()
    }
    /// must be used from the gtk main context
    pub fn set_time(&self, time: chrono::DateTime<Local>) {
        let old_time = self.get_time();
        self.imp().time.replace(time);
        if time.minute() != old_time.minute() {
            self.queue_draw();
        }
    }
}
