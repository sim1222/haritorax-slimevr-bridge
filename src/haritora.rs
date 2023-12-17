use crate::math::{Gravity, Rotation};
use byteorder::{LittleEndian as LE, ReadBytesExt};
use std::io::Cursor;
use uuid::{uuid, Uuid};

#[derive(Debug, Clone, Copy)]
pub enum Characteristics {
    Battery,
    SoftwareRevision,
    Sensor,
    Magnetormeter,
    MainButton,
    SecondaryButton,
    FpsSetting,
    TofSetting,
    SensorModeSetting,
    WirelessModeSetting,
    AutoCalibrationSetting,
}

impl Into<Uuid> for Characteristics {
    fn into(self) -> Uuid {
        match self {
            Self::Battery => uuid!("00002a19-0000-1000-8000-00805f9b34fb"),
            Self::SoftwareRevision => uuid!("00002a28-0000-1000-8000-00805f9b34fb"),
            Self::Sensor => uuid!("00dbf1c6-90aa-11ed-a1eb-0242ac120002"),
            Self::Magnetormeter => uuid!("00dbf306-90aa-11ed-a1eb-0242ac120002"),
            Self::MainButton => uuid!("00dbf450-90aa-11ed-a1eb-0242ac120002"),
            Self::SecondaryButton => uuid!("00dbf586-90aa-11ed-a1eb-0242ac120002"),
            Self::FpsSetting => uuid!("ef844202-90a9-11ed-a1eb-0242ac120002"),
            Self::TofSetting => uuid!("ef8443f6-90a9-11ed-a1eb-0242ac120002"),
            Self::SensorModeSetting => uuid!("ef8445c2-90a9-11ed-a1eb-0242ac120002"),
            Self::WirelessModeSetting => uuid!("ef84c300-90a9-11ed-a1eb-0242ac120002"),
            Self::AutoCalibrationSetting => uuid!("ef84c305-90a9-11ed-a1eb-0242ac120002"),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Services {
    Tracker,
    Setting,
    Battery,
    DeviceInfo,
}

impl Into<Uuid> for Services {
    fn into(self) -> Uuid {
        match self {
            Self::Tracker => uuid!("00dbec3a-90aa-11ed-a1eb-0242ac120002"),
            Self::Setting => uuid!("ef84369a-90a9-11ed-a1eb-0242ac120002"),
            Self::Battery => uuid!("0000180f-0000-1000-8000-00805f9b34fb"),
            Self::DeviceInfo => uuid!("0000180a-0000-1000-8000-00805f9b34fb"),
        }
    }
}

#[derive(Debug)]
pub(crate) enum DecodeError {
    TooFewBytes,
}

const E: DecodeError = DecodeError::TooFewBytes;

/// This function returns 0~1.0 floating point value
pub fn decode_battery_packet(data: &[u8]) -> Result<f32, DecodeError> {
    let b = data.first().ok_or(E)?;

    Ok(*b as f32 / 100.0)
}

pub fn decode_imu_packet(data: &[u8]) -> Result<(Rotation, Gravity), DecodeError> {
    let mut cur = Cursor::new(data);

    let rotation = Rotation {
        x: cur.read_i16::<LE>().map_err(|_| E)? as f32 * 0.01,
        y: cur.read_i16::<LE>().map_err(|_| E)? as f32 * 0.01,
        z: cur.read_i16::<LE>().map_err(|_| E)? as f32 * 0.01 * -1.0,
        w: cur.read_i16::<LE>().map_err(|_| E)? as f32 * 0.01 * -1.0,
    };

    let gravity = Gravity {
        x: cur.read_i16::<LE>().map_err(|_| E)? as f32 / 256f32,
        y: cur.read_i16::<LE>().map_err(|_| E)? as f32 / 256f32,
        z: cur.read_i16::<LE>().map_err(|_| E)? as f32 / 256f32,
    };

    Ok((rotation, gravity))
}
