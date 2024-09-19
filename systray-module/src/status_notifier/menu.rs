use std::{
    collections::HashMap,
    fmt::Display,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use zbus::{names::BusName, zvariant::Value};

use super::{
    layout::{Layout, LayoutChild, LayoutProperty},
    proxy::dbus_menu::DbusMenuProxy,
};

#[derive(Debug, Clone, Copy)]
pub enum Status {
    Normal,
    Notice,
}

impl std::str::FromStr for Status {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "normal" => Ok(Status::Normal),
            "notice" => Ok(Status::Notice),
            _ => Err(()),
        }
    }
}

impl Display for Status {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Status::Normal => write!(f, "normal"),
            Status::Notice => write!(f, "notice"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Event {
    Clicked,
    Hovered,
}

impl std::str::FromStr for Event {
    type Err = ();

    fn from_str(s: &str) -> std::result::Result<Self, ()> {
        match s {
            "clicked" => Ok(Event::Clicked),
            "hovered" => Ok(Event::Hovered),
            _ => Err(()),
        }
    }
}

impl Display for Event {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Clicked => write!(f, "clicked"),
            Event::Hovered => write!(f, "hovered"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Menu {
    /// The StatusNotifierItem that is wrapped by this instance.
    pub dm: DbusMenuProxy<'static>,
}

impl Menu {
    /// Create a new instance of the Menu struct.
    pub async fn from_address(
        conn: &zbus::Connection,
        address: &BusName<'_>,
        object_path: zbus::zvariant::OwnedObjectPath,
    ) -> zbus::Result<Self> {
        let dm = DbusMenuProxy::builder(conn)
            .destination(address.to_string())?
            .path(object_path)?
            .build()
            .await?;
        Ok(Self { dm })
    }
}

#[allow(dead_code)]
impl Menu {
    // Properties

    pub async fn version(&self) -> zbus::Result<u32> {
        self.dm.version().await
    }

    pub async fn status(&self) -> zbus::Result<Status> {
        let status = self.dm.status().await?;
        match status.parse() {
            Ok(s) => Ok(s),
            Err(_) => Err(zbus::Error::Failure(format!("Invalid status {:?}", status))),
        }
    }

    pub async fn icon_theme_path(&self) -> zbus::Result<Vec<String>> {
        self.dm.icon_theme_path().await
    }

    // Methods

    pub async fn get_layout_root(&self) -> zbus::Result<Layout> {
        Layout::from_dm(&self.dm).await
    }

    pub async fn get_layout(
        &self,
        parent_id: i32,
        recursion_depth: i32,
        property_names: Vec<&str>,
    ) -> zbus::Result<LayoutChild> {
        let (_, layout_child) = self
            .dm
            .get_layout(parent_id, recursion_depth, &property_names)
            .await?;
        LayoutChild::try_from(layout_child)
            .map_err(|err| zbus::Error::Failure(format!("Invalid layout {:?}", err)))
    }

    pub async fn get_group_properties(
        &self,
        ids: &[i32],
        property_names: &[&str],
    ) -> zbus::Result<HashMap<i32, LayoutChild>> {
        let res = self.dm.get_group_properties(ids, property_names).await?;
        let mut children = HashMap::new();
        for (id, child_properties) in res {
            let mut child = LayoutChild {
                id,
                properties: HashMap::new(),
                children: Vec::new(),
            };
            for (key, value) in child_properties {
                let property = LayoutProperty::try_from((key.as_str(), value))
                    .map_err(|err| zbus::Error::Failure(format!("Invalid property {:?}", err)))?;
                child.properties.insert(key, property);
            }
            children.insert(id, child);
        }
        Ok(children)
    }

    pub async fn get_property(&self, id: i32, property_name: &str) -> zbus::Result<LayoutProperty> {
        let value = self.dm.get_property(id, property_name).await?;
        LayoutProperty::try_from((property_name, value))
            .map_err(|err| zbus::Error::Failure(format!("Invalid property {:?}", err)))
    }

    pub async fn event(&self, id: i32, event: Event, data: Option<&Value<'_>>) -> zbus::Result<()> {
        self.dm
            .event(
                id,
                &event.to_string(),
                data.unwrap_or(&Value::I32(0)),
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or(Duration::ZERO)
                    .as_millis() as u32,
            )
            .await
    }

    pub async fn about_to_show(&self, id: i32) -> zbus::Result<bool> {
        self.dm.about_to_show(id).await
    }
}
