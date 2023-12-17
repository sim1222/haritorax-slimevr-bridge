use std::{
    io::{Cursor, Write},
    sync::atomic::AtomicU64,
};

use byteorder::{LittleEndian, ReadBytesExt};
use nalgebra::Vector3;
use tokio::net::UdpSocket;

use crate::{
    constants::constants::TxPacketType,
    math::{gravity::Gravity, rotation::Rotation},
};

pub async fn handle_battery_data(data: &[u8], socket: &UdpSocket, packet_count: &AtomicU64) {
    let mut cur = Cursor::new(data);

    let battery_level = cur.read_u8().unwrap() as f32 * 0.01;

    println!("Battery level: {}", battery_level);

    let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level

    let count = packet_count.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let _ = buf
        .write(&u32::from(TxPacketType::BatteryLevel).to_be_bytes())
        .unwrap();
    let _ = buf.write(&count.to_be_bytes()).unwrap();
    let _ = buf.write(&battery_level.to_be_bytes()).unwrap();

    socket.send(buf.get_ref()).await.unwrap();
}

pub async fn handle_imu_data(data: &[u8], socket: &UdpSocket, packet_count: &AtomicU64) {
    let mut cur = Cursor::new(data);

    let rotation = Rotation {
        x: cur.read_i16::<LittleEndian>().unwrap() as f32 * 0.01,
        y: cur.read_i16::<LittleEndian>().unwrap() as f32 * 0.01,
        z: cur.read_i16::<LittleEndian>().unwrap() as f32 * 0.01 * -1.0,
        w: cur.read_i16::<LittleEndian>().unwrap() as f32 * 0.01 * -1.0,
    };
    // let rotation = Quaternion::new(rotation.w, rotation.x, rotation.y, rotation.z);

    let gravity = Gravity {
        x: cur.read_u8().unwrap() as f32 * 0.01 + cur.read_i8().unwrap() as f32,
        y: cur.read_u8().unwrap() as f32 * 0.01 + cur.read_i8().unwrap() as f32,
        z: cur.read_u8().unwrap() as f32 * 0.01 + cur.read_i8().unwrap() as f32,
    };

    let gravity = Vector3::new(gravity.x, gravity.y, gravity.z);

    // println!("Rotation: {:?}", rotation);
    // println!("Gyro: {:?}", gyro);
    // println!("IMU data: {:?}", cur.get_ref());

    // rotation
    let mut buf = Cursor::new([0u8; 12 + 4 * 4]); // 12 header, f32 * 4 rotation

    // let mut packet_count = packet_count.lock().await;
    //
    // *packet_count = packet_count.wrapping_add(1);
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
