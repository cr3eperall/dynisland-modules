use std::{cell::RefCell, str::FromStr};

use chrono::{DateTime, Days, Local, Timelike};
use dynisland_core::abi::{gdk, glib, gtk, log};
use gdk::RGBA;
use glib::{
    subclass::{
        object::{DerivedObjectProperties, ObjectImpl, ObjectImplExt},
        types::{ObjectSubclass, ObjectSubclassExt},
    },
    Object, Properties,
};
use gtk::{
    graphene::Rect,
    prelude::*,
    subclass::widget::{WidgetClassExt, WidgetImpl},
    BinLayout,
};
use pangocairo::glib::subclass::types::ObjectSubclassIsExt;

use crate::upower::device::HistoryEntry;

glib::wrapper! {
    pub struct Graph(ObjectSubclass<GraphPriv>)
    @extends gtk::Widget;
}
#[derive(Properties)]
#[properties(wrapper_type = Graph)]
pub struct GraphPriv {
    #[property(get, set)]
    max_duration_secs: RefCell<u32>,
    #[property(get, set)]
    draw_bars: RefCell<bool>,

    points: RefCell<Vec<HistoryEntry>>,
}

#[glib::object_subclass]
impl ObjectSubclass for GraphPriv {
    const NAME: &'static str = "BatteryGraphWidget";
    type Type = Graph;
    type ParentType = gtk::Widget;

    fn class_init(klass: &mut Self::Class) {
        klass.set_layout_manager_type::<BinLayout>();
        klass.set_css_name("battery-graph-widget");
    }
}
#[allow(clippy::derivable_impls)]
impl Default for GraphPriv {
    fn default() -> Self {
        Self {
            max_duration_secs: RefCell::new(36000),
            draw_bars: RefCell::new(false),
            points: RefCell::new(Vec::new()),
        }
    }
}

#[glib::derived_properties]
impl ObjectImpl for GraphPriv {
    fn constructed(&self) {
        self.parent_constructed();
        // let battery = self.obj().clone();
        // glib::timeout_add_local(Duration::from_millis(100), move || {
        //     battery.queue_draw();
        //     glib::ControlFlow::Continue
        // });
    }

    fn dispose(&self) {
        while let Some(child) = self.obj().first_child() {
            child.unparent();
        }
    }

    fn set_property(&self, _id: usize, value: &glib::Value, pspec: &glib::ParamSpec) {
        match pspec.name() {
            "max-duration-secs" => {
                let mut max_duration_secs: u32 = value.get().unwrap();
                let points = self.points.borrow();
                let min_point = points.iter().min_by_key(|entry| entry.timestamp);
                if let Some(min_point) = min_point {
                    let now = Local::now().timestamp() as u32;
                    let max_dur = now - min_point.timestamp;
                    if max_dur < max_duration_secs
                        && max_duration_secs < self.max_duration_secs.borrow().clone()
                    {
                        max_duration_secs = max_dur;
                    }
                }
                self.max_duration_secs
                    .replace(max_duration_secs.max(60 * 4));
                self.obj().queue_draw();
            }
            "draw-bars" => {
                let draw_bars: bool = value.get().unwrap();
                self.draw_bars.replace(draw_bars);
                self.obj().queue_draw();
            }
            // "fill-color" => {
            //     let name: String = value.get().unwrap();
            //     if let Ok(color) = RGBA::parse(&name) {
            //         self.fill_color.replace(color);
            //         self.obj().queue_draw();
            //     } else {
            //         log::warn!("invalid fill color: {name}");
            //     }
            // }
            // "background-color" => {
            //     let name: String = value.get().unwrap();
            //     if let Ok(color) = RGBA::parse(&name) {
            //         self.background_color.replace(color);
            //         self.obj().queue_draw();
            //     } else {
            //         log::warn!("invalid background color: {name}");
            //     }
            // }
            // "percentage" => {
            //     let percentage: f64 = value.get().unwrap();
            //     self.percentage.replace(percentage);
            //     self.obj().queue_draw();
            // }
            // "percentage-markup" => {
            //     let pango_markup: String = value.get().unwrap();
            //     self.percentage_markup.replace(pango_markup);
            //     self.obj().queue_draw();
            // }
            // "charging" => {
            //     let charging: bool = value.get().unwrap();
            //     self.charging.replace(charging);
            //     self.obj().queue_draw();
            // }
            // "show-percentage" => {
            //     let show: bool = value.get().unwrap();
            //     self.show_percentage.replace(show);
            //     self.obj().queue_draw();
            // }
            _ => {
                log::warn!("Battery: invalid property received: {}", pspec.name());
            }
        }
    }

    fn property(&self, id: usize, pspec: &glib::ParamSpec) -> glib::Value {
        match pspec.name() {
            "max-duration-secs" => self.max_duration_secs.borrow().to_value(),
            "draw-bars" => self.draw_bars.borrow().to_value(),
            // "fill-color" => self.fill_color.borrow().to_string().to_value(),
            // "background-color" => self.background_color.borrow().to_string().to_value(),
            // "percentage" => self.percentage.borrow().to_value(),
            // "charging" => self.charging.borrow().to_value(),
            // "show-percentage" => self.show_percentage.borrow().to_value(),
            _ => self.derived_property(id, pspec),
        }
    }
}

impl WidgetImpl for GraphPriv {
    fn snapshot(&self, snapshot: &gtk::Snapshot) {
        let (w, h) = (self.obj().width() as f64, self.obj().height() as f64);
        let w_h_ratio = 1.1;
        let mut req_h = h;
        let mut req_w = req_h * w_h_ratio;
        // let radius = 4.0;
        if req_w > w {
            req_w = w;
            req_h = req_w / w_h_ratio;
        }

        let (x, y) = ((w - req_w) / 2.0, (h - req_h) / 2.0);
        let rect = Rect::new(x as f32, y as f32, req_w as f32, req_h as f32);

        let ctx = snapshot.append_cairo(&rect);
        // let bg_color = self.background_color.borrow();

        // add space to not clip the border (1px around)
        ctx.translate(x + 1.0, y + 1.0);
        let req_w = req_w - 2.0;
        let req_h = req_h - 2.0;

        let main_w = req_w * 0.9 - 20.0;
        let main_h = req_h - 40.0;
        ctx.translate(req_w * 0.05 + 20.0, 10.0);

        let now = Local::now().timestamp() as u32;

        let mut range_points: Vec<HistoryEntry> = Vec::new();
        let points = self.points.borrow();

        let min_point = points.iter().min_by_key(|entry| entry.timestamp);
        let max_point = points.iter().max_by_key(|entry| entry.timestamp);

        let min_limit = now - *self.max_duration_secs.borrow();
        let min_limit = min_point.map_or(min_limit, |entry| min_limit.max(entry.timestamp));

        let max_limit = now;

        let mins = (max_limit - min_limit) / 60;

        let step = match mins {
            0..5 => 1,         // 1 minute
            5..10 => 2,        // 2 minutes
            10..25 => 5,       // 5 minutes
            25..50 => 10,      // 10 minutes
            50..100 => 20,     // 20 minutes
            100..300 => 60,    // 1 hour
            300..600 => 120,   // 2 hours
            600..1200 => 240,  // 4 hours
            1200..2400 => 480, // 8 hours
            2400..3600 => 720, // 12 hours
            _ => 1440,         // 1 day
        };
        // align to minute or hour or day
        let min_dt: DateTime<Local> = chrono::DateTime::from_timestamp(min_limit as i64, 0)
            .unwrap()
            .into();
        // log::debug!("min_pt: {}, step: {}", min_pt.format("%H:%M"), step);
        let minutes = (min_dt.num_seconds_from_midnight() + 1) / 60;
        let rounded_minute = step + minutes - (minutes % step); // round to the next step
                                                                // log::debug!(
                                                                //     "step: {}, minutes: {}, rounded_minute: {}",
                                                                //     step,
                                                                //     minutes,
                                                                //     rounded_minute
                                                                // );
                                                                // log::debug!("rounded_minute: {}", rounded_minute);
        let min_instant = min_dt
            .with_hour((rounded_minute % 1440) / 60)
            .unwrap()
            .with_minute(rounded_minute % 60)
            .unwrap()
            .with_second(0)
            .unwrap()
            .checked_add_days(Days::new((rounded_minute / 1440).into()))
            .unwrap()
            .timestamp() as u32;

        // remove points outside the range

        for entry in points.iter() {
            if entry.timestamp < min_limit || entry.timestamp > max_limit {
                continue;
            }
            range_points.push(*entry);
        }
        if let Some(max) = max_point {
            let mut new_max = max.clone();
            new_max.timestamp = now;
            range_points.push(new_max)
        }

        // draw points
        if mins != 0 {
            draw_points(&ctx, main_w, main_h, range_points, min_limit, mins);
        }

        ctx.set_font_size(12.0);
        draw_grid_vertical(
            &ctx,
            min_instant,
            min_limit,
            max_limit,
            step,
            main_w,
            main_h,
            *self.draw_bars.borrow(),
        );
        draw_grid_horizontal(&ctx, main_w, main_h, *self.draw_bars.borrow());

        drop(ctx);
    }
}

fn draw_points(
    ctx: &gtk::cairo::Context,
    main_w: f64,
    main_h: f64,
    points: Vec<HistoryEntry>,
    min_limit: u32,
    mins: u32,
) {
    ctx.set_source_rgba(0.0, 0.9, 0.2, 1.0);
    ctx.move_to(0.0, main_h);
    let mut last_jump = 0.0;
    let mut last_x = 0.0;
    let mut last_perc = -1.0;
    for HistoryEntry {
        timestamp: instant,
        value: percentage,
        state: _,
    } in points.into_iter()
    {
        let x = match instant.checked_sub(min_limit) {
            Some(x) => x,
            None => {
                log::warn!("instant: {} < min_limit: {}", instant, min_limit);
                continue;
            }
        };

        let x = x as f64 / 60.0;
        let x = x / mins as f64; // percentage
        let x = x * main_w;
        let y = main_h * (1.0 - percentage / 100.0);
        if last_perc == -1.0 {
            last_perc = percentage;
            last_jump = x;
            ctx.line_to(0.0, y);
        }
        // log::debug!("p: {}% y: {}", percentage, y);

        if percentage == 0.0 && (percentage - last_perc).abs() > 10.0 {
            ctx.line_to(last_x, main_h);
            ctx.line_to(last_jump, main_h);
            ctx.fill().unwrap();
            ctx.move_to(x, y);
            last_jump = x;
        } else if last_perc == 0.0 && (percentage - last_perc).abs() > 10.0 {
            ctx.move_to(last_x, main_h);
            ctx.line_to(x, y);
            // ctx.stroke().unwrap();
            last_jump = x;
        } else {
            // ctx.rectangle(x - 2.0, y, 4.0, main_h - y);
            // ctx.fill().unwrap();
            ctx.line_to(x, y);
        }
        last_perc = percentage;
        last_x = x;
    }

    ctx.line_to(last_x, main_h);
    ctx.line_to(last_jump, main_h);
    ctx.fill().unwrap();
}

fn draw_grid_vertical(
    ctx: &gtk::cairo::Context,
    min_instant: u32,
    min_limit: u32,
    max_limit: u32,
    step: u32,
    main_w: f64,
    main_h: f64,
    draw_bars: bool,
) {
    let grid_v = (min_instant..max_limit)
        .step_by((step * 60) as usize)
        .collect::<Vec<_>>();

    // log::debug!("grid_v: {}: {:?}", grid_v.len(), grid_v);

    // draw grid vertical
    for instant in grid_v.iter() {
        let x = match instant.checked_sub(min_limit) {
            Some(x) => x,
            None => {
                // log::warn!("instant2: {} < min_limit: {}", instant, min_limit);
                continue;
            }
        };
        let x = x as f64 / 60.0;
        let x = x / ((max_limit - min_limit) / 60) as f64; // percentage
        let x = x * main_w;
        if draw_bars {
            ctx.move_to(x, main_h);
            ctx.line_to(x, 0.0);
            ctx.set_source_color(&RGBA::from_str("gray").unwrap().with_alpha(0.5));
            ctx.stroke().unwrap();
        }
        let date_time: DateTime<Local> = chrono::DateTime::from_timestamp(*instant as i64, 0)
            .unwrap()
            .into();
        let mut y_offset = 5.0;
        // TODO allow for date and time format customization
        let formatted_date = if step >= 720 {
            Some(date_time.format("%d/%m").to_string())
        } else {
            None
        };
        let formatted_time = if step != 1440 {
            Some(date_time.format("%H:%M").to_string())
        } else {
            None
        };

        ctx.set_source_color(&RGBA::WHITE);
        if let Some(formatted) = formatted_time {
            let ext = ctx.text_extents(&formatted).unwrap();
            y_offset += ext.height();
            ctx.move_to(x - (ext.width() / 2.0), main_h + y_offset);
            ctx.show_text(&formatted).unwrap();
            y_offset += 4.0;
        }
        if let Some(formatted) = formatted_date {
            let ext = ctx.text_extents(&formatted).unwrap();
            y_offset += ext.height();
            ctx.move_to(x - (ext.width() / 2.0), main_h + y_offset);
            ctx.show_text(&formatted).unwrap();
        }
    }
}

fn draw_grid_horizontal(ctx: &gtk::cairo::Context, main_w: f64, main_h: f64, draw_bars: bool) {
    let grid_h = (0..=100).step_by(25 as usize).collect::<Vec<_>>();

    // log::debug!("grid_h: {}: {:?}", grid_h.len(), grid_h);

    // draw grid vertical
    for perc in grid_h.iter() {
        let y = 1.0 - (*perc as f64 / 100.0);
        let y = y * main_h;
        if draw_bars {
            ctx.move_to(0.0, y);
            ctx.line_to(main_w, y);
            ctx.set_source_color(&RGBA::from_str("gray").unwrap().with_alpha(0.5));
            ctx.stroke().unwrap();
        }

        let perc = format!("{:.0}% ", perc);

        ctx.set_source_color(&RGBA::WHITE);
        let ext = ctx.text_extents(&perc).unwrap();
        ctx.move_to(-ext.width(), y + ext.height() / 2.0);
        ctx.show_text(&perc).unwrap();
    }
}

impl GraphPriv {}

#[allow(clippy::new_without_default)]
impl Graph {
    pub fn new() -> Self {
        let this: Self = Object::builder().build();
        this
    }
    /// This assumes the points are sorted by increasing time
    pub fn set_points(&self, points: &Vec<HistoryEntry>) {
        self.imp().points.replace(points.to_vec());
        self.queue_draw();
    }
}
