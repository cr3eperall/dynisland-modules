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
