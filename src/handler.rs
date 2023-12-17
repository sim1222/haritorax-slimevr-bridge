use std::{
    io::{Cursor, Write},
    sync::atomic::AtomicU64,
};

use nalgebra::Vector3;
use tokio::net::UdpSocket;

use crate::{
    constants::constants::TxPacketType,
    math::{Gravity, Rotation},
};

pub async fn handle_battery_data(battery_level: f32, socket: &UdpSocket, packet_count: &AtomicU64) {
    println!("Battery level: {battery_level}");

    let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level

    let count = packet_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = buf
        .write(&u32::from(TxPacketType::BatteryLevel).to_be_bytes())
        .unwrap();
    let _ = buf.write(&count.to_be_bytes()).unwrap();
    let _ = buf.write(&battery_level.to_be_bytes()).unwrap();

    socket.send(buf.get_ref()).await.unwrap();
}

pub async fn handle_imu_data(
    rotation: Rotation,
    gravity: Gravity,
    socket: &UdpSocket,
    packet_count: &AtomicU64,
) {
    let gravity = Vector3::new(gravity.x, gravity.y, gravity.z);

    // println!("Rotation: {:?}", rotation);
    // println!("Gyro: {:?}", gyro);
    // println!("IMU data: {:?}", cur.get_ref());

    // rotation
    let mut buf = Cursor::new([0u8; 12 + 4 * 4]); // 12 header, f32 * 4 rotation
    let count = packet_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let _ = buf
        .write(&u32::from(TxPacketType::Rotation).to_be_bytes())
        .unwrap();

    let _ = buf.write(&count.to_be_bytes()).unwrap();
    let _ = buf.write(&rotation.x.to_be_bytes()).unwrap();
    let _ = buf.write(&rotation.y.to_be_bytes()).unwrap();
    let _ = buf.write(&rotation.z.to_be_bytes()).unwrap();
    let _ = buf.write(&rotation.w.to_be_bytes()).unwrap();

    // println!(
    //     "Send: {:?} | Type: {:?} | Count: {:?}",
    //     buf.get_ref(),
    //     buf.get_ref()[0],
    //     packet_count
    // );
    socket.send(buf.get_ref()).await.unwrap();

    // accel
    let mut buf = Cursor::new([0u8; 12 + 4 * 3]); // 12 header, f32 * 3 accel

    let count = packet_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let _ = buf
        .write(&u32::from(TxPacketType::Accel).to_be_bytes())
        .unwrap();
    let _ = buf.write(&count.to_be_bytes()).unwrap();
    let _ = buf.write(&gravity.x.to_be_bytes()).unwrap();
    let _ = buf.write(&gravity.y.to_be_bytes()).unwrap();
    let _ = buf.write(&gravity.z.to_be_bytes()).unwrap();

    socket.send(buf.get_ref()).await.unwrap();
}
