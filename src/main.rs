use btleplug::api::ValueNotification;
use btleplug::api::{
    bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use constants::constants::PACKET_EOF;
use futures::stream::StreamExt;
use nalgebra::{Quaternion, Rotation3, Vector3};
use rand::prelude::*;
use std::io::Cursor;
use std::net::SocketAddr;
use std::{
    error::Error,
    io::{BufRead as _, Read, Write},
    net::{Ipv4Addr, UdpSocket},
    sync::{Arc, Mutex},
};
use tokio::time;
use uuid::{uuid, Uuid};

mod constants;
mod math;
mod utils;

use crate::constants::characteristics::{
    BATTERY_CHARACTERISTIC, MAIN_BUTTON_CHARACTERISTIC, SENSOR_CHARACTERISTIC,
};
use crate::constants::constants::{
    CURRENT_VERSION, PACKET_ACCEL, PACKET_BATTERY_LEVEL, PACKET_HANDSHAKE, PACKET_ROTATION,
};
use crate::math::gravity::Gravity;
use crate::math::rotation::Rotation;
use crate::math::*;
use crate::utils::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();

    let adapters = manager.adapters().await.unwrap();
    let central = adapters.into_iter().nth(0).unwrap();

    central.start_scan(ScanFilter::default()).await.unwrap();

    println!("Scanning for 5 seconds...");

    time::sleep(std::time::Duration::from_millis(5000)).await;

    let tracker = find_tracker(&central)
        .await
        .expect("Could not find tracker");

    tracker.connect().await.unwrap();

    println!(
        "Connected to tracker: {:?}",
        tracker.properties().await.unwrap().unwrap().local_name
    );

    let mac = tracker.properties().await.unwrap().unwrap().address;
    let mac_bytes: [u8; 6] = mac.as_ref().try_into().unwrap(); // Convert the mac address to a byte array

    tracker.discover_services().await.unwrap();

    let chars = tracker.characteristics();

    let imu_data = chars
        .clone()
        .into_iter()
        .find(|c| c.uuid == Uuid::parse_str(SENSOR_CHARACTERISTIC).unwrap())
        .unwrap();

    let battery_level = chars
        .clone()
        .into_iter()
        .find(|c| c.uuid == Uuid::parse_str(BATTERY_CHARACTERISTIC).unwrap())
        .unwrap();

    let main_button = chars
        .clone()
        .into_iter()
        .find(|c| c.uuid == Uuid::parse_str(MAIN_BUTTON_CHARACTERISTIC).unwrap())
        .unwrap();

    let target = "192.168.1.3:6969";
    // let target = "255.255.255.255:6969";

    let mut rng = rand::thread_rng();

    let port = rng.gen_range(10000..20000);

    let socket =
        UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], port))).expect("Could not bind socket");

    // socket.set_broadcast(true).expect("Could not set broadcast");
    socket
        .set_read_timeout(Some(std::time::Duration::from_millis(2500)))
        .unwrap();
    socket
        .set_write_timeout(Some(std::time::Duration::from_millis(2500)))
        .unwrap();

    try_handshake(&socket, mac_bytes, target).await.unwrap();

    let packet_count = Arc::new(Mutex::new(0u64));

    tracker.subscribe(&imu_data).await.unwrap();
    tracker.subscribe(&battery_level).await.unwrap();
    tracker.subscribe(&main_button).await.unwrap();

    let mut notifications = tracker.notifications().await.unwrap();

    let sock = socket.try_clone().unwrap();
    let packet_c = packet_count.clone();
    tokio::spawn(async move { handle_slime_packet(&sock, &packet_c) });

    // main loop for notificated data from the tracker
    loop {
        let data = notifications.next().await.unwrap();

        match data
            .uuid
            .hyphenated()
            .encode_lower(&mut Uuid::encode_buffer())
        {
            uuid if uuid == SENSOR_CHARACTERISTIC.to_owned().as_str() => {
                handle_imu_data(&data.value, &socket, &packet_count);
            }
            uuid if uuid == BATTERY_CHARACTERISTIC.to_owned().as_str() => {
                handle_battery_data(&data.value, &socket, &packet_count);
            }
            uuid if uuid == MAIN_BUTTON_CHARACTERISTIC.to_owned().as_str() => {
                println!("Received button push");
            }
            _ => {
                println!("Received unknown data from tracker");
            }
        }
    }
}

fn handle_slime_packet(socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
    loop {
        let mut buf = [0u8; 256];
        socket.recv_from(buf.as_mut()).unwrap();
        parse_packet(&buf, &mut packet_count.lock().unwrap(), &socket);
    }
}

fn handle_battery_data(data: &Vec<u8>, socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
    let mut cur = Cursor::new(data);

    let battery_level = cur.read_u8().unwrap() as f32 * 0.01;

    println!("Battery level: {}", battery_level);

    let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level

    let mut packet_count = packet_count.lock().unwrap();

    *packet_count = packet_count.wrapping_add(1);

    buf.write(PACKET_BATTERY_LEVEL.to_be_bytes().as_ref())
        .unwrap();
    buf.write(packet_count.to_be_bytes().as_ref()).unwrap();
    buf.write(battery_level.to_be_bytes().as_ref()).unwrap();

    socket.send(buf.get_ref()).unwrap();
}

fn handle_imu_data(data: &Vec<u8>, socket: &UdpSocket, packet_count: &Arc<Mutex<u64>>) {
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

    let mut packet_count = packet_count.lock().unwrap();

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
    socket.send(buf.get_ref()).unwrap();

    // accel
    let mut buf = Cursor::new([0u8; 12 + 4 * 3]); // 12 header, f32 * 3 accel

    *packet_count = packet_count.wrapping_add(1);

    buf.write(PACKET_ACCEL.to_be_bytes().as_ref()).unwrap();
    buf.write(packet_count.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.x.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.y.to_be_bytes().as_ref()).unwrap();
    buf.write(gravity.z.to_be_bytes().as_ref()).unwrap();

    socket.send(buf.get_ref()).unwrap();
}

async fn find_tracker(central: &Adapter) -> Option<Peripheral> {
    for peripheral in central.peripherals().await.unwrap() {
        // println!(
        //     "Found peripheral with name {:?}",
        //     peripheral.properties().await.unwrap().unwrap().local_name
        // );
        if peripheral
            .properties()
            .await
            .unwrap()
            .unwrap()
            .local_name
            .iter()
            .any(|name| name.starts_with("HaritoraXW"))
        {
            return Some(peripheral);
        }
    }
    None
}

fn insert_slime_info(buf: &mut Cursor<[u8; 66]>, mac: [u8; 6]) {
    let board_type: u32 = 13;
    let imu_type: u32 = 0; // other
    let mcu_type: u32 = 3; // esp32

    // slimeVR version > 8 uses sensor info packet instead of imu info
    // it is kept here for backwards compatibility

    let imu_info: [u32; 3] = [0, 0, 0];

    let firmware_version_number: u32 = 8;

    let firmware_version = "HaritoraX-Wireless".as_bytes();
    let firmware_version_len = firmware_version.len() as u8;

    let mac_address = mac;

    println!("Mac address: {:?}", bytes_to_hex_string(&mac_address));

    buf.write(board_type.to_be_bytes().as_ref()).unwrap();
    buf.write(imu_type.to_be_bytes().as_ref()).unwrap();
    buf.write(mcu_type.to_be_bytes().as_ref()).unwrap();

    for imu in imu_info.iter() {
        buf.write(imu.to_be_bytes().as_ref()).unwrap();
    }

    buf.write(firmware_version_number.to_be_bytes().as_ref())
        .unwrap();

    buf.write(firmware_version_len.to_be_bytes().as_ref()).unwrap();

    buf.write(firmware_version.as_ref()).unwrap();

    buf.write(mac_address.as_ref()).unwrap();

    buf.write(PACKET_EOF.to_be_bytes().as_ref()).unwrap();
}

async fn try_handshake(
    socket: &UdpSocket,
    mac: [u8; 6],
    target: &str,
) -> Result<(), Box<dyn Error>> {
    // let mut cur = Cursor::new([0 as u8; 12 + 36 + 9]); // 12 header, 36 slime info, 9 footer
    let mut cur = Cursor::new([0 as u8; 12 + 45 + 9]); // 12 header, 36 slime info, 9 footer

    cur.write(PACKET_HANDSHAKE.to_be_bytes().as_ref()).unwrap(); // handshake packet
    cur.write(0u64.to_be_bytes().as_ref()).unwrap(); // handshake packet number

    insert_slime_info(&mut cur, mac); // Pass the converted value

    // println!("{:?}", cur.get_ref());

    socket.send_to(cur.get_ref(), target).unwrap();

    println!("Sent packet");

    let mut buf = [0u8; 64];

    let (amt, src) = socket
        .recv_from(&mut buf)
        .expect("Could not receive packet");

    let mut buf_cursor = Cursor::new(buf);

    if buf_cursor.get_ref()[0] != PACKET_HANDSHAKE as u8 {
        panic!("Received packet with wrong type from {}", src);
    }

    buf_cursor.set_position(1);

    let buf = &mut [0u8; 12];
    buf_cursor.read_exact(buf).unwrap();

    if buf.starts_with("Hey OVR =D".as_bytes()) {
        println!("Received handshake packet from {}", src);
    } else {
        panic!("Received packet with wrong content from {}", src);
    }

    let server_version: i32 = std::str::from_utf8(buf)
        .unwrap()
        .chars()
        .skip(11)
        .take_while(|c| c.is_numeric())
        .collect::<String>()
        .parse()
        .unwrap();

    println!("Server version: {}", server_version);

    if server_version != CURRENT_VERSION {
        panic!("Server version does not match client version");
    }

    socket.connect(src).unwrap();

    println!("Connected to {}", src);

    Ok(())
}
