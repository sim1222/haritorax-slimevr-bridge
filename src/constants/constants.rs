use num_enum::{IntoPrimitive, TryFromPrimitive};

pub const PACKET_EOF: u8 = 0xFF;

pub const CURRENT_VERSION: i32 = 5;

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum TxPacketType {
    Heartbeat = 0,
    Rotation = 1,
    Gyro = 2,
    Handshake = 3,
    Accel = 4,
    Mag = 5,
    RawCalibrationData = 6,
    CalibrationFinished = 7,
    Config = 8,
    RawMagentometer = 9,
    Serial = 11,
    BatteryLevel = 12,
    Tap = 13,
    ResetReason = 14,
    SensorInfo = 15,
    Rotation2 = 16,
    RotationData = 17,
    MagentometerAccuracy = 18,
    ButtonPushed = 60,
    SendMagStatus = 61,
}

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
pub enum RxPacketType {
    Heartbeat = 1,
    Vibrate = 2,
    Handshake = 3,
    Command = 4,
    PingPong = 10,
    ChangeMagStatus = 62,
}
