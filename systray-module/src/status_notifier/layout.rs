use std::{
    collections::{HashMap, HashSet},
    error::Error,
};

use dynisland_core::abi::{
    gdk::{self, gdk_pixbuf::Pixbuf},
    glib::Bytes,
    gtk::{self, gio::MemoryInputStream},
};
use zbus::zvariant::OwnedValue;

use super::proxy::dbus_menu::{DbusMenuProxy, ItemsPropertiesUpdatedArgs};

#[derive(Debug, Clone)]
pub enum TypeProperty {
    Standard,
    Separator,
    Vendor(String),
}

impl From<&str> for TypeProperty {
    fn from(s: &str) -> Self {
        match s {
            "standard" => TypeProperty::Standard,
            "separator" => TypeProperty::Separator,
            _ => TypeProperty::Vendor(s.to_owned()),
        }
    }
}
#[derive(Debug, Clone, Copy, Default)]
pub enum ToggleProperty {
    #[default]
    None,
    Checkmark,
    Radio,
}

impl From<&str> for ToggleProperty {
    fn from(s: &str) -> Self {
        match s {
            "checkmark" => ToggleProperty::Checkmark,
            "radio" => ToggleProperty::Radio,
            _ => ToggleProperty::None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayoutParseError {
    InvalidType(String),
}

impl LayoutParseError {
    pub fn invalid_type(prop: &str, err: impl Error) -> Self {
        LayoutParseError::InvalidType(format!("prop: {}, err: {:?}", prop, err))
    }
}

pub const LAYOUT_PROP_TYPE: &str = "type";
pub const LAYOUT_PROP_LABEL: &str = "label";
pub const LAYOUT_PROP_ENABLED: &str = "enabled";
pub const LAYOUT_PROP_VISIBLE: &str = "visible";
pub const LAYOUT_PROP_ICON_NAME: &str = "icon-name";
pub const LAYOUT_PROP_ICON_DATA: &str = "icon-data";
pub const LAYOUT_PROP_SHORTCUT: &str = "shortcut";
pub const LAYOUT_PROP_TOGGLE_TYPE: &str = "toggle-type";
pub const LAYOUT_PROP_TOGGLE_STATE: &str = "toggle-state";
pub const LAYOUT_PROP_CHILDREN_DISPLAY: &str = "children-display";

#[derive(Debug)]
pub enum LayoutProperty {
    Type(TypeProperty),
    Label(String),
    Enabled(bool),
    Visible(bool),
    IconName(String),
    IconData(gdk::Paintable),
    Shortcut(Vec<Vec<String>>),
    ToggleType(ToggleProperty),
    ToggleState(bool),
    ChildrenDisplay(bool),
    Vendor(zbus::zvariant::OwnedValue),
}

impl Clone for LayoutProperty {
    fn clone(&self) -> Self {
        match self {
            Self::Type(arg0) => Self::Type(arg0.clone()),
            Self::Label(arg0) => Self::Label(arg0.clone()),
            Self::Enabled(arg0) => Self::Enabled(arg0.clone()),
            Self::Visible(arg0) => Self::Visible(arg0.clone()),
            Self::IconName(arg0) => Self::IconName(arg0.clone()),
            Self::IconData(arg0) => Self::IconData(arg0.clone()),
            Self::Shortcut(arg0) => Self::Shortcut(arg0.clone()),
            Self::ToggleType(arg0) => Self::ToggleType(arg0.clone()),
            Self::ToggleState(arg0) => Self::ToggleState(arg0.clone()),
            Self::ChildrenDisplay(arg0) => Self::ChildrenDisplay(arg0.clone()),
            Self::Vendor(arg0) => Self::Vendor(arg0.try_clone().unwrap()),
        }
    }
}

impl TryFrom<(&str, zbus::zvariant::OwnedValue)> for LayoutProperty {
    type Error = LayoutParseError;

    fn try_from(value: (&str, zbus::zvariant::OwnedValue)) -> Result<Self, Self::Error> {
        let (key, value) = value;
        match key {
            LAYOUT_PROP_TYPE => {
                let value_str: String = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_TYPE, err))?;
                let type_ = TypeProperty::from(value_str.as_str());
                Ok(LayoutProperty::Type(type_))
            }
            LAYOUT_PROP_LABEL => {
                let value_str: String = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_LABEL, err))?;
                Ok(LayoutProperty::Label(value_str))
            }
            LAYOUT_PROP_ENABLED => {
                let value_bool: bool = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_ENABLED, err))?;
                Ok(LayoutProperty::Enabled(value_bool))
            }
            LAYOUT_PROP_VISIBLE => {
                let value_bool: bool = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_VISIBLE, err))?;
                Ok(LayoutProperty::Visible(value_bool))
            }
            LAYOUT_PROP_ICON_NAME => {
                let value_str: String = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_ICON_NAME, err))?;
                Ok(LayoutProperty::IconName(value_str))
            }
            LAYOUT_PROP_ICON_DATA => {
                let value_vec: Vec<u8> = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_ICON_DATA, err))?;
                let data = value_vec.as_slice();
                let data = Bytes::from(data);
                let mut pixbuf = Pixbuf::from_stream(
                    &MemoryInputStream::from_bytes(&data),
                    None::<&gtk::gio::Cancellable>,
                )
                .ok();
                if pixbuf.is_none() {
                    pixbuf = Pixbuf::new(gdk::gdk_pixbuf::Colorspace::Rgb, true, 8, 10, 10);
                }
                let texture = gdk::Texture::for_pixbuf(&pixbuf.unwrap());
                Ok(LayoutProperty::IconData(texture.into()))
            }
            LAYOUT_PROP_SHORTCUT => {
                let value_vec: Vec<Vec<String>> = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_SHORTCUT, err))?;
                Ok(LayoutProperty::Shortcut(value_vec))
            }
            LAYOUT_PROP_TOGGLE_TYPE => {
                let value_str: String = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_TOGGLE_TYPE, err))?;
                let toggle_type = ToggleProperty::from(value_str.as_str());
                Ok(LayoutProperty::ToggleType(toggle_type))
            }
            LAYOUT_PROP_TOGGLE_STATE => {
                let value_int: i32 = value
                    .try_into()
                    .map_err(|err| LayoutParseError::invalid_type(LAYOUT_PROP_TOGGLE_STATE, err))?;
                // TODO add indeterminate state
                match value_int {
                    1 => Ok(LayoutProperty::ToggleState(true)),
                    _ => Ok(LayoutProperty::ToggleState(false)),
                }
            }
            LAYOUT_PROP_CHILDREN_DISPLAY => {
                let value_str: String = value.try_into().map_err(|err| {
                    LayoutParseError::invalid_type(LAYOUT_PROP_CHILDREN_DISPLAY, err)
                })?;
                match value_str.as_str() {
                    "submenu" => Ok(LayoutProperty::ChildrenDisplay(true)),
                    _ => Ok(LayoutProperty::ChildrenDisplay(false)),
                }
            }
            _ => Ok(LayoutProperty::Vendor(value)),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct LayoutChild {
    pub id: i32,
    pub properties: HashMap<String, LayoutProperty>,
    pub children: Vec<LayoutChild>,
}

impl TryFrom<(i32, HashMap<String, OwnedValue>, Vec<OwnedValue>)> for LayoutChild {
    type Error = LayoutParseError;

    fn try_from(
        value: (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>),
    ) -> Result<Self, Self::Error> {
        let (id, properties, children) = value;

        let mut properties_map = HashMap::new();
        for (key, value) in properties {
            let parsed_value = LayoutProperty::try_from((key.as_str(), value))?;
            properties_map.insert(key, parsed_value);
        }

        let mut children_vec = Vec::new();
        for child_value in children {
            let child: (i32, HashMap<String, OwnedValue>, Vec<OwnedValue>) = child_value
                .try_into()
                .map_err(|err| LayoutParseError::invalid_type("child", err))?;
            let child = LayoutChild::try_from(child)?;
            children_vec.push(child);
        }

        Ok(LayoutChild {
            id,
            properties: properties_map,
            children: children_vec,
        })
    }
}

impl LayoutChild {
    pub fn get_ids_mut(&mut self, update_ids: Vec<i32>) -> HashMap<i32, &mut LayoutChild> {
        if update_ids.contains(&self.id) {
            return HashMap::from([(self.id, self)]);
        }
        if self.children.is_empty() {
            return HashMap::new();
        }
        let mut remaining_children = HashSet::new();
        remaining_children.extend(update_ids);
        let mut children = HashMap::new();
        for child in self.children.iter_mut() {
            if remaining_children.is_empty() {
                break;
            }
            if remaining_children.remove(&child.id) {
                children.insert(child.id, child);
                continue;
            }
            let found_ids = child.get_ids_mut(remaining_children.iter().copied().collect());
            for (id, child) in found_ids {
                remaining_children.remove(&id);
                children.insert(id, child);
            }
        }
        children
    }
}
#[derive(Debug, Clone, Default)]
pub struct Layout {
    pub revision: u32,
    pub root: LayoutChild,
}

impl Layout {
    pub async fn from_dm(dm: &DbusMenuProxy<'_>) -> zbus::Result<Self> {
        let (revision, layout_root) = dm.get_layout(0, -1, &vec![]).await?;
        let root = LayoutChild::try_from(layout_root)
            .map_err(|err| zbus::Error::Failure(format!("Invalid layout {:?}", err)))?;
        Ok(Layout { revision, root })
    }

    pub async fn update(
        &mut self,
        dm: &DbusMenuProxy<'_>,
        update_ids: Vec<i32>,
    ) -> zbus::Result<()> {
        let is_root = update_ids.contains(&0);
        if is_root {
            let new = Self::from_dm(dm).await?;
            self.revision = new.revision;
            self.root = new.root;
            return Ok(());
        }
        let mut children_to_update = self.root.get_ids_mut(update_ids);
        let ids = children_to_update.keys().copied().collect::<Vec<i32>>();
        let children = dm.get_group_properties(ids.as_slice(), &vec![]).await?;
        let mut children_to_refresh = Vec::new();
        for (id, child_properties) in children {
            let old_child = match children_to_update.remove(&id) {
                Some(child) => child,
                None => continue,
            };
            old_child.children.clear();
            old_child.properties.clear();
            for (key, value) in child_properties {
                let parsed_value = LayoutProperty::try_from((key.as_str(), value))
                    .map_err(|err| zbus::Error::Failure(format!("Invalid property {:?}", err)))?;
                if let LayoutProperty::ChildrenDisplay(true) = parsed_value {
                    children_to_refresh.push(old_child);
                    break;
                }
                old_child.properties.insert(key, parsed_value);
            }
        }
        for child in children_to_refresh {
            let (revision, refreshed_child) = dm.get_layout(child.id, -1, &vec![]).await?;
            self.revision = revision;
            let refreshed_child = LayoutChild::try_from(refreshed_child)
                .map_err(|err| zbus::Error::Failure(format!("Invalid layout {:?}", err)))?;
            *child = refreshed_child;
        }
        Ok(())
    }

    pub async fn update_child(&mut self, dm: &DbusMenuProxy<'_>, id: i32) -> zbus::Result<()> {
        let (revision, layout_child) = dm.get_layout(id, -1, &vec![]).await?;
        self.revision = revision;
        let layout_child = LayoutChild::try_from(layout_child)
            .map_err(|err| zbus::Error::Failure(format!("Invalid layout {:?}", err)))?;
        let mut children_to_update = self.root.get_ids_mut(vec![id]);
        if let Some(child) = children_to_update.remove(&id) {
            *child = layout_child;
        }
        Ok(())
    }
    pub async fn update_props(&mut self, msg: &ItemsPropertiesUpdatedArgs<'_>) -> zbus::Result<()> {
        let ItemsPropertiesUpdatedArgs {
            updated_props,
            removed_props,
            ..
        } = msg;
        let updated_ids: Vec<i32> = updated_props.iter().map(|(id, _)| *id).collect();
        let removed_ids: Vec<i32> = removed_props.iter().map(|(id, _)| *id).collect();
        let mut all_ids = updated_ids.clone();
        all_ids.extend(removed_ids);
        let children = self.root.get_ids_mut(all_ids);
        for (child_id, child) in children {
            if let Some((_, updated_props)) = updated_props.iter().find(|(id, _)| *id == child_id) {
                for (key, value) in updated_props {
                    let owned_value = OwnedValue::try_from(value)?;
                    let parsed_value = LayoutProperty::try_from((key.as_ref(), owned_value))
                        .map_err(|err| {
                            zbus::Error::Failure(format!("Invalid property {:?}", err))
                        })?;
                    child.properties.insert(key.to_string(), parsed_value);
                }
            } else if let Some((_, removed_props)) =
                removed_props.iter().find(|(id, _)| *id == child_id)
            {
                for key in removed_props {
                    child.properties.remove(AsRef::<str>::as_ref(key));
                }
            }
        }
        Ok(())
    }
}
