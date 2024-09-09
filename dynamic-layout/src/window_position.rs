use dynisland_core::abi::{gdk, gtk, gtk_layer_shell, log};
use gdk::prelude::*;
use gtk::{prelude::*, Window};
use gtk_layer_shell::LayerShell;
use serde::{Deserialize, Serialize};

use crate::config::WindowPosition;

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(tag = "Alignment")]
pub enum Alignment {
    #[serde(alias = "start")]
    Start,
    #[serde(alias = "center")]
    Center,
    #[serde(alias = "end")]
    End,
}

impl Alignment {
    pub fn map_gtk(&self) -> gtk::Align {
        match self {
            Alignment::Start => gtk::Align::Start,
            Alignment::Center => gtk::Align::Center,
            Alignment::End => gtk::Align::End,
        }
    }
}

#[derive(Serialize, Deserialize, Default, Debug, Clone)]
#[serde(tag = "Layer")]
pub enum Layer {
    #[serde(alias = "background")]
    Background,
    #[serde(alias = "bottom")]
    Bottom,
    #[default]
    #[serde(alias = "top")]
    Top,
    #[serde(alias = "overlay")]
    Overlay,
}

impl Layer {
    pub fn map_gtk(&self) -> gtk_layer_shell::Layer {
        match self {
            Layer::Background => gtk_layer_shell::Layer::Background,
            Layer::Bottom => gtk_layer_shell::Layer::Bottom,
            Layer::Top => gtk_layer_shell::Layer::Top,
            Layer::Overlay => gtk_layer_shell::Layer::Overlay,
        }
    }
}

impl WindowPosition {
    pub fn config_layer_shell_for(&self, window: &Window) {
        window.set_layer(self.layer.map_gtk());
        match self.v_anchor {
            Alignment::Start => {
                window.set_anchor(gtk_layer_shell::Edge::Top, true);
                window.set_anchor(gtk_layer_shell::Edge::Bottom, false);
                window.set_margin(gtk_layer_shell::Edge::Top, self.margin_y);
            }
            Alignment::Center => {
                window.set_anchor(gtk_layer_shell::Edge::Top, false);
                window.set_anchor(gtk_layer_shell::Edge::Bottom, false);
            }
            Alignment::End => {
                window.set_anchor(gtk_layer_shell::Edge::Top, false);
                window.set_anchor(gtk_layer_shell::Edge::Bottom, true);
                window.set_margin(gtk_layer_shell::Edge::Bottom, self.margin_y);
            }
        }
        match self.h_anchor {
            Alignment::Start => {
                window.set_anchor(gtk_layer_shell::Edge::Left, true);
                window.set_anchor(gtk_layer_shell::Edge::Right, false);
                window.set_margin(gtk_layer_shell::Edge::Left, self.margin_x);
            }
            Alignment::Center => {
                window.set_anchor(gtk_layer_shell::Edge::Left, false);
                window.set_anchor(gtk_layer_shell::Edge::Right, false);
            }
            Alignment::End => {
                window.set_anchor(gtk_layer_shell::Edge::Left, false);
                window.set_anchor(gtk_layer_shell::Edge::Right, true);
                window.set_margin(gtk_layer_shell::Edge::Right, self.margin_x);
            }
        }
        let mut monitor = None;
        for mon in gdk::Display::default()
            .unwrap()
            .monitors()
            .iter::<gdk::Monitor>()
        {
            let mon = match mon {
                Ok(monitor) => monitor,
                Err(_) => {
                    continue;
                }
            };
            if mon.connector().unwrap().eq_ignore_ascii_case(&self.monitor) {
                monitor = Some(mon);
                break;
            }
        }
        if let Some(monitor) = monitor {
            window.set_monitor(&monitor);
        }
        window.set_namespace("dynisland");
        window.set_exclusive_zone(self.exclusive_zone);
        window.set_resizable(false);
        window.queue_resize();
    }

    pub fn init_window(&self, window: &Window) {
        if self.layer_shell {
            window.init_layer_shell();
            self.config_layer_shell_for(window.upcast_ref());
            window.connect_destroy(|_| log::debug!("LayerShell window was destroyed"));
        } else {
            window.connect_destroy(|_| std::process::exit(0));
        }
    }
    pub fn reconfigure_window(&self, window: &Window) {
        if self.layer_shell {
            self.config_layer_shell_for(window);
        }
    }
}
