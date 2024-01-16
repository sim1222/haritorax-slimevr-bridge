use std::io::{Cursor, Read, Write};

use tokio::net::UdpSocket;

use crate::math::{Gravity, Rotation};
use num_enum::{IntoPrimitive, TryFromPrimitive};
use std::str::FromStr;

const CURRENT_VERSION: i32 = 5;

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
enum TxPacketType {
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
    SignalStrength = 19,
    Temperature = 20,
    UserAction = 21,
    ButtonPushed = 60,
    SendMagStatus = 61,
}

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u32)]
enum RxPacketType {
    Heartbeat = 1,
    Vibrate = 2,
    Handshake = 3,
    Command = 4,
    PingPong = 10,
    ChangeMagStatus = 62,
}

#[derive(IntoPrimitive, TryFromPrimitive)]
#[repr(u8)]
pub enum UserActionType {
    ResetFull = 2,
    ResetYaw = 3,
    ResetMounting = 4,
    PauseTracking = 5,
}

#[derive(Debug, Clone)]
pub struct BoardInfo {
    board_type: u32,
    imu_type: u32,
    mcu_type: u32,
    imu_info: [u32; 3],
    firmware_version_number: u32,
    firmware_version: String,
    mac_addr: [u8; 6],
}

impl Default for BoardInfo {
    fn default() -> Self {
        Self {
            board_type: 13,
            imu_type: 0,
            mcu_type: 3,
            imu_info: [0, 0, 0],
            firmware_version_number: 0,
            firmware_version: "".to_string(),
            mac_addr: [0, 0, 0, 0, 0, 0],
        }
    }
}

impl BoardInfo {
    pub fn new(mac_addr: &[u8; 6]) -> Self {
        let mut b = Self::default();
        b.mac_addr = mac_addr.to_owned();
        b
    }

    #[must_use]
    pub fn board_type(&self, board_type: u32) -> Self {
        let mut b = self.clone();
        b.board_type = board_type;
        b
    }

    #[must_use]
    pub fn imu_type(&self, imu_type: u32) -> Self {
        let mut b = self.clone();
        b.imu_type = imu_type;
        b
    }

    #[must_use]
    pub fn mcu_type(&self, mcu_type: u32) -> Self {
        let mut b = self.clone();
        b.mcu_type = mcu_type;
        b
    }

    #[must_use]
    pub fn imu_info(&self, imu_info: [u32; 3]) -> Self {
        let mut b = self.clone();
        b.imu_info = imu_info;
        b
    }

    #[must_use]
    pub fn firmware_version_number(&self, firmware_version_number: u32) -> Self {
        let mut b = self.clone();
        b.firmware_version_number = firmware_version_number;
        b
    }

    #[must_use]
    pub fn firmware_version(&self, firmware_version: &str) -> Self {
        let mut b = self.clone();
        b.firmware_version = firmware_version.to_string();
        b
    }

    #[must_use]
    pub fn mac_addr(&self, mac_addr: &[u8; 6]) -> Self {
        let mut b = self.clone();
        b.mac_addr = mac_addr.to_owned();
        b
    }
}

pub fn write_handshake_packet<W: Write>(mut buf: W, b: &BoardInfo) {
    let _ = buf
        .write(&u32::from(TxPacketType::Handshake).to_be_bytes())
        .unwrap();
    let _ = buf.write(&0u64.to_be_bytes()).unwrap();

    let _ = buf.write(&b.board_type.to_be_bytes()).unwrap();
    let _ = buf.write(&b.imu_type.to_be_bytes()).unwrap();
    let _ = buf.write(&b.mcu_type.to_be_bytes()).unwrap();

    for imu in b.imu_info.iter() {
        let _ = buf.write(&imu.to_be_bytes()).unwrap();
    }

    let _ = buf.write(&b.firmware_version_number.to_be_bytes()).unwrap();
    let _ = buf.write(&[b.firmware_version.len() as u8]).unwrap();
    let _ = buf.write(&b.firmware_version.as_bytes()).unwrap();
    let _ = buf.write(&b.mac_addr).unwrap();

    // EOF
    let _ = buf.write(&[0xFFu8]).unwrap();
}

#[derive(Debug)]
struct SafePacketNumberGenerator(u64);

impl SafePacketNumberGenerator {
    fn new() -> Self {
        SafePacketNumberGenerator(0)
    }

    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add_signed(1);
        self.0
    }
}

#[derive(Debug)]
pub enum ClientError {
    UdpSocketError(std::io::Error),
    SendHandShakePacket(std::io::Error),
    ReceiveHandShakePacket(std::io::Error),
}

#[derive(Debug)]
pub struct Client {
    socket: UdpSocket,
    packet_number: SafePacketNumberGenerator,
}

impl Client {
    pub async fn try_new(socket: UdpSocket, b: &BoardInfo) -> Result<Self, ClientError> {
        socket
            .set_broadcast(true)
            .map_err(ClientError::UdpSocketError)?;

        let mut cur = Cursor::new(vec![]);
        write_handshake_packet(&mut cur, b);
        socket
            .send_to(cur.get_ref(), "255.255.255.255:6969")
            .await
            .map_err(ClientError::SendHandShakePacket)?;

        let mut buf = [0u8; 4096];

        let src = loop {
            let (_, src) = socket
                .recv_from(&mut buf)
                .await
                .map_err(ClientError::ReceiveHandShakePacket)?;

            if buf[0] != u32::from(RxPacketType::Handshake) as u8 {
                continue;
            }

            if !buf[1..].starts_with(b"Hey OVR =D") {
                continue;
            }

            let version: Vec<u8> = buf.into_iter().filter(|v| v.is_ascii_digit()).collect();
            let Ok(version) = std::str::from_utf8(&version) else {
                continue;
            };

            let Ok(version) = i32::from_str(version) else {
                continue;
            };

            if version != CURRENT_VERSION {
                continue;
            }

            break src;
        };

        socket.connect(src).await.unwrap();

        Ok(Self {
            socket,
            packet_number: SafePacketNumberGenerator::new(),
        })
    }

    pub async fn try_send_rotation(&mut self, rot: &Rotation) -> Result<(), std::io::Error> {
        let mut buf = Cursor::new([0u8; 12 + 4 * 4]); // 12 header, f32 * 4 rotation

        let _ = buf
            .write(&u32::from(TxPacketType::Rotation).to_be_bytes())
            .unwrap();

        let _ = buf.write(&self.packet_number.next().to_be_bytes()).unwrap();
        let _ = buf.write(&rot.x.to_be_bytes()).unwrap();
        let _ = buf.write(&rot.y.to_be_bytes()).unwrap();
        let _ = buf.write(&rot.z.to_be_bytes()).unwrap();
        let _ = buf.write(&rot.w.to_be_bytes()).unwrap();

        self.socket.send(buf.get_ref()).await?;

        Ok(())
    }

    pub async fn recv(&mut self) {
        let mut buf = [9u8; 4096];
        let Ok(_) = self.socket.recv_from(&mut buf).await else {
            return;
        };

        let mut packet = Cursor::new(buf);

        let mut packet_type = [0u8; 4];

        let Ok(_) = packet.read_exact(&mut packet_type) else {
            return;
        };

        let packet_type = RxPacketType::try_from(u32::from_be_bytes(packet_type));

        match packet_type {
            Ok(RxPacketType::Heartbeat) => {
                // println!("Received heartbeat")
            },
            Ok(RxPacketType::Vibrate) => println!("Received vibrate"),
            Ok(RxPacketType::PingPong) => {
                self.socket.send(packet.get_ref()).await.unwrap();
                // println!("Received ping pong");
            }
            Ok(RxPacketType::Handshake) => unreachable!("Unexpected Handshake packet"),
            Ok(RxPacketType::Command) => println!("Received command"),
            Ok(RxPacketType::ChangeMagStatus) => {
                // TODO
                println!("こけっちが書く");
            }
            Err(e) => println!("Received unknown packet type {e}"),
        }
    }

    pub async fn try_send_gravity(&mut self, gravity: &Gravity) -> Result<(), std::io::Error> {
        let mut buf = Cursor::new([0u8; 12 + 4 * 3]); // 12 header, f32 * 3 accel

        let _ = buf
            .write(&u32::from(TxPacketType::Accel).to_be_bytes())
            .unwrap();
        let _ = buf.write(&self.packet_number.next().to_be_bytes()).unwrap();
        let _ = buf.write(&gravity.x.to_be_bytes()).unwrap();
        let _ = buf.write(&gravity.y.to_be_bytes()).unwrap();
        let _ = buf.write(&gravity.z.to_be_bytes()).unwrap();

        self.socket.send(buf.get_ref()).await.unwrap();

        Ok(())
    }

    pub async fn try_send_mag_enabled(&mut self, enabled: bool) -> Result<(), std::io::Error> {
        let mut buf = Cursor::new([0u8; 12 + 1]); // 12 header, 1 mag enabled

        let _ = buf
            .write(&u32::from(TxPacketType::SendMagStatus).to_be_bytes())
            .unwrap();
        let _ = buf.write(&self.packet_number.next().to_be_bytes()).unwrap();
        let _ = buf.write(if enabled { b"y" } else { b"n" }).unwrap();

        self.socket.send(buf.get_ref()).await.unwrap();

        Ok(())
    }

    pub async fn try_send_battery_level(
        &mut self,
        battery_level: f32,
    ) -> Result<(), std::io::Error> {
        let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level

        let _ = buf
            .write(&u32::from(TxPacketType::BatteryLevel).to_be_bytes())
            .unwrap();
        let _ = buf.write(&self.packet_number.next().to_be_bytes()).unwrap();
        let _ = buf.write(&battery_level.to_be_bytes()).unwrap();

        self.socket.send(buf.get_ref()).await.unwrap();
        Ok(())
    }

    pub async fn try_send_user_action(&mut self, user_action: u8) -> Result<(), std::io::Error> {
        let mut buf = Cursor::new([0u8; 12 + 1]); // 12 header, 1 user action

        let _ = buf
            .write(&u32::from(TxPacketType::UserAction).to_be_bytes())
            .unwrap();
        let _ = buf.write(&self.packet_number.next().to_be_bytes()).unwrap();
        let _ = buf.write(&user_action.to_be_bytes()).unwrap();

        self.socket.send(buf.get_ref()).await.unwrap();
        Ok(())
    }
}
