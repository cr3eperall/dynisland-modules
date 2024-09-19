//! Implementation of the StatusNotifierItem and dbusmenu protocols.
//!
//! The proxy module was taken from [eww](https://github.com/elkowar/eww/tree/master/crates/notifier_host/src/proxy)
//!
//! `host.rs`, `icon.rs`, `item.rs` and `watcher.rs` were taken from [eww](https://github.com/elkowar/eww/tree/master/crates/notifier_host/src/proxy)
//!  and slightly modified for use with gtk4 and dynisland.
pub mod host;
pub mod icon;
pub mod item;
pub mod layout;
pub mod menu;
pub mod proxy;
pub mod watcher;

pub(crate) mod names {
    pub const WATCHER_BUS: &str = "org.kde.StatusNotifierWatcher";
    pub const WATCHER_OBJECT: &str = "/StatusNotifierWatcher";

    pub const ITEM_OBJECT: &str = "/StatusNotifierItem";
}
