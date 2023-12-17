use std::{
    io::{Cursor, Write},
    sync::Arc,
};

use byteorder::{LittleEndian, ReadBytesExt};
use nalgebra::Vector3;
use tokio::{sync::Mutex, net::UdpSocket};

use crate::{
    constants::constants::{PACKET_ACCEL, PACKET_BATTERY_LEVEL, PACKET_ROTATION},
    math::{gravity::Gravity, rotation::Rotation},
    utils::parse_packet,
};

pub async fn handle_slime_packet(socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
    loop {
        let mut buf = [0u8; 256];
        socket.recv_from(buf.as_mut()).await.unwrap();
        parse_packet(&buf, &mut *packet_count.lock().await, &socket);
    }
}

pub async fn handle_battery_data(data: &Vec<u8>, socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
    let mut cur = Cursor::new(data);

    let battery_level = cur.read_u8().unwrap() as f32 * 0.01;

    println!("Battery level: {}", battery_level);

    let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level

    let mut packet_count = packet_count.lock().await;

    *packet_count = packet_count.wrapping_add(1);

    buf.write(PACKET_BATTERY_LEVEL.to_be_bytes().as_ref())
        .unwrap();
    buf.write(packet_count.to_be_bytes().as_ref()).unwrap();
    buf.write(battery_level.to_be_bytes().as_ref()).unwrap();

    socket.send(buf.get_ref()).await.unwrap();
}

pub async fn handle_imu_data(data: &Vec<u8>, socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
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

    let mut packet_count = packet_count.lock().await;

    *packet_count = packet_count.wrapping_add(1);

    buf.write(PACKET_ROTATION.to_be_bytes().as_ref()).unwrap();
    buf.write(packet_count.to_be_bytes().as_ref()).unwrap();
    buf.write(rotation.x.to_be_bytes().as_ref()).unwrap();
    buf.write(rotation.y.to_be_bytes().as_ref()).unwrap();
    buf.write(rotation.z.to_be_bytes().as_ref()).unwrap();
    buf.write(rotation.w.to_be_bytes().as_ref()).unwrap();

    // println!(
    //     "Send: {:?} | Type: {:?} | Count: {:?}",
    //     buf.get_ref(),
    //     buf.get_ref()[0],
    //     packet_count
    // );
    socket.send(buf.get_ref()).await.unwrap();

    // accel
    let mut buf = Cursor::new([0u8; 12 + 4 * 3]); // 12 header, f32 * 3 accel

    *packet_count = packet_count.wrapping_add(1);

    buf.write(PACKET_ACCEL.to_be_bytes().as_ref()).unwrap();
    buf.write(packet_count.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.x.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.y.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.z.to_be_bytes().as_ref()).unwrap();

    socket.send(buf.get_ref()).await.unwrap();
}
