use std::{
    fmt::Display,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use zbus::zvariant::ObjectPath;

use super::proxy::device::DeviceProxy;

#[derive(Debug, Clone, Copy)]
pub enum HistoryType {
    Rate,
    Charge,
}

impl Display for HistoryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryType::Rate => write!(f, "rate"),
            HistoryType::Charge => write!(f, "charge"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum StatisticsType {
    Charging,
    Discharging,
}

impl Display for StatisticsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StatisticsType::Charging => write!(f, "charging"),
            StatisticsType::Discharging => write!(f, "discharging"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum State {
    Unknown = 0,
    Charging = 1,
    Discharging = 2,
    Empty = 3,
    FullyCharged = 4,
    PendingCharge = 5,
    PendingDischarge = 6,
}

impl From<u32> for State {
    fn from(val: u32) -> Self {
        match val {
            0 => State::Unknown,
            1 => State::Charging,
            2 => State::Discharging,
            3 => State::Empty,
            4 => State::FullyCharged,
            5 => State::PendingCharge,
            6 => State::PendingDischarge,
            _ => State::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HistoryEntry {
    pub timestamp: u32,
    pub value: f64,
    pub state: State,
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct StatisticsEntry {
    pub value: f64,
    pub accuracy: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum BatteryLevel {
    Unknown = 0,
    None = 1,
    Unknown2 = 2,
    Low = 3,
    Critical = 4,
    Unknown5 = 5,
    Normal = 6,
    High = 7,
    Full = 8,
}

impl From<u32> for BatteryLevel {
    fn from(val: u32) -> Self {
        match val {
            0 => BatteryLevel::Unknown,
            1 => BatteryLevel::None,
            2 => BatteryLevel::Unknown2,
            3 => BatteryLevel::Low,
            4 => BatteryLevel::Critical,
            5 => BatteryLevel::Unknown5,
            6 => BatteryLevel::Normal,
            7 => BatteryLevel::High,
            8 => BatteryLevel::Full,
            _ => BatteryLevel::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Technology {
    Unknown = 0,
    LithiumIon = 1,
    LithiumPolymer = 2,
    LithiumIronPhosphate = 3,
    LeadAcid = 4,
    NickelCadmium = 5,
    NickelMetalHydride = 6,
}

impl From<u32> for Technology {
    fn from(val: u32) -> Self {
        match val {
            0 => Technology::Unknown,
            1 => Technology::LithiumIon,
            2 => Technology::LithiumPolymer,
            3 => Technology::LithiumIronPhosphate,
            4 => Technology::LeadAcid,
            5 => Technology::NickelCadmium,
            6 => Technology::NickelMetalHydride,
            _ => Technology::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WarningLevel {
    Unknown = 0,
    None = 1,
    Discharging = 2,
    Low = 3,
    Critical = 4,
    Action = 5,
}

impl From<u32> for WarningLevel {
    fn from(val: u32) -> Self {
        match val {
            0 => WarningLevel::Unknown,
            1 => WarningLevel::None,
            2 => WarningLevel::Discharging,
            3 => WarningLevel::Low,
            4 => WarningLevel::Critical,
            5 => WarningLevel::Action,
            _ => WarningLevel::Unknown,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum DeviceType {
    Unknown = 0,
    LinePower = 1,
    Battery = 2,
    Ups = 3,
    Monitor = 4,
    Mouse = 5,
    Keyboard = 6,
    Pda = 7,
    Phone = 8,
    MediaPlayer = 9,
    Tablet = 10,
    Computer = 11,
    GamingInput = 12,
    Pen = 13,
    Touchpad = 14,
    Modem = 15,
    Network = 16,
    Headset = 17,
    Speakers = 18,
    Headphones = 19,
    Video = 20,
    OtherAudio = 21,
    RemoteControl = 22,
    Printer = 23,
    Scanner = 24,
    Camera = 25,
    Wearable = 26,
    Toy = 27,
    BluetoothGeneric = 28,
}

impl From<u32> for DeviceType {
    fn from(val: u32) -> Self {
        match val {
            0 => DeviceType::Unknown,
            1 => DeviceType::LinePower,
            2 => DeviceType::Battery,
            3 => DeviceType::Ups,
            4 => DeviceType::Monitor,
            5 => DeviceType::Mouse,
            6 => DeviceType::Keyboard,
            7 => DeviceType::Pda,
            8 => DeviceType::Phone,
            9 => DeviceType::MediaPlayer,
            10 => DeviceType::Tablet,
            11 => DeviceType::Computer,
            12 => DeviceType::GamingInput,
            13 => DeviceType::Pen,
            14 => DeviceType::Touchpad,
            15 => DeviceType::Modem,
            16 => DeviceType::Network,
            17 => DeviceType::Headset,
            18 => DeviceType::Speakers,
            19 => DeviceType::Headphones,
            20 => DeviceType::Video,
            21 => DeviceType::OtherAudio,
            22 => DeviceType::RemoteControl,
            23 => DeviceType::Printer,
            24 => DeviceType::Scanner,
            25 => DeviceType::Camera,
            26 => DeviceType::Wearable,
            27 => DeviceType::Toy,
            28 => DeviceType::BluetoothGeneric,
            _ => DeviceType::Unknown,
        }
    }
}

/// A upower Device
///
/// you should directly access the `proxy` member as needed for functionalty that is not provided.
#[derive(Debug, Clone)]
pub struct Device {
    /// The Device that is wrapped by this instance.
    pub proxy: DeviceProxy<'static>,
}

impl Device {
    /// Create an instance from the service's address.
    ///
    /// # Arguments
    ///
    /// * `conn` - The connection to the D-Bus
    /// * `path` - The path to the device
    pub async fn from_path<P>(conn: &zbus::Connection, path: P) -> zbus::Result<Self>
    where
        P: TryInto<ObjectPath<'static>>,
        P::Error: Into<zbus::Error>,
    {
        let proxy = DeviceProxy::builder(conn)
            .destination("org.freedesktop.UPower")?
            .path(path)?
            .build()
            .await?;
        Ok(Self { proxy })
    }
}

impl Device {
    /// GetHistory method
    ///
    /// * `type_` - The type of history to get
    /// * `timespan` - The amount of data to return in seconds, or 0 for all.
    /// * `resolution` - The approximate number of points to return.
    ///        A higher resolution is more accurate, at the expense of plotting speed.
    pub async fn get_history(
        &self,
        type_: HistoryType,
        timespan: u32,
        resolution: u32,
    ) -> zbus::Result<Vec<HistoryEntry>> {
        let type_ = type_.to_string();
        let entries = self.proxy.get_history(&type_, timespan, resolution).await?;
        let mut result = Vec::new();
        for (timestamp, value, state) in entries {
            result.push(HistoryEntry {
                timestamp,
                value,
                state: State::from(state),
            });
        }
        // reverse the list so that the oldest entry is first
        result.reverse();
        Ok(result)
    }

    #[allow(dead_code)]
    pub async fn get_statistics(
        &self,
        type_: StatisticsType,
    ) -> zbus::Result<Vec<StatisticsEntry>> {
        let type_ = type_.to_string();
        let entries = self.proxy.get_statistics(&type_).await?;
        let mut result = Vec::new();
        for (value, accuracy) in entries {
            result.push(StatisticsEntry { value, accuracy });
        }
        Ok(result)
    }

    pub async fn battery_level(&self) -> zbus::Result<BatteryLevel> {
        let level = self.proxy.battery_level().await?;
        Ok(BatteryLevel::from(level))
    }
    pub async fn state(&self) -> zbus::Result<State> {
        let state = self.proxy.state().await?;
        Ok(State::from(state))
    }
    pub async fn technology(&self) -> zbus::Result<Technology> {
        let tech = self.proxy.technology().await?;
        Ok(Technology::from(tech))
    }
    pub async fn time_to_empty(&self) -> zbus::Result<Duration> {
        let time = self.proxy.time_to_empty().await?;
        if time < 0 {
            return Err(zbus::Error::Failure(
                "time to empty is negative".to_string(),
            ));
        }
        Ok(Duration::from_secs(time as u64))
    }
    pub async fn time_to_full(&self) -> zbus::Result<Duration> {
        let time = self.proxy.time_to_full().await?;
        if time < 0 {
            return Err(zbus::Error::Failure("time to full is negative".to_string()));
        }
        Ok(Duration::from_secs(time as u64))
    }
    pub async fn type_(&self) -> zbus::Result<DeviceType> {
        let type_ = self.proxy.type_().await?;
        Ok(DeviceType::from(type_))
    }

    #[allow(dead_code)]
    pub async fn update_time(&self) -> zbus::Result<SystemTime> {
        let time = self.proxy.update_time().await?;
        let time = UNIX_EPOCH
            .checked_add(Duration::from_secs(time as u64))
            .ok_or(zbus::Error::Failure(
                "failed to convert time from unix epoch".to_string(),
            ))?;
        Ok(time)
    }

    #[allow(dead_code)]
    pub async fn warning_level(&self) -> zbus::Result<WarningLevel> {
        let level = self.proxy.warning_level().await?;
        Ok(WarningLevel::from(level))
    }
}
