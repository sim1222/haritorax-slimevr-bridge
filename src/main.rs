use btleplug::api::{
    bleuuid::uuid_from_u16, Central, Manager as _, Peripheral as _, ScanFilter, WriteType,
};
use btleplug::platform::{Adapter, Manager, Peripheral};
use byteorder::{BigEndian, ByteOrder, LittleEndian, ReadBytesExt};
use futures::stream::StreamExt;
use nalgebra::{Quaternion, Rotation3, Vector3};
use rand::prelude::*;
use std::fmt::Debug;
use std::io::Cursor;
use std::{
    error::Error,
    io::{BufRead as _, Read, Write},
    net::{Ipv4Addr, UdpSocket},
    sync::{Arc, Mutex},
};
use tokio::time;
use uuid::{uuid, Uuid};

const PACKET_EOF: u8 = 0xFF;

const CURRENT_VERSION: i32 = 5;

const PACKET_HEARTBEAT: u32 = 0;
const PACKET_ROTATION: u32 = 1;
const PACKET_GYRO: u32 = 2;
const PACKET_HANDSHAKE: u32 = 3;
const PACKET_ACCEL: u32 = 4;
const PACKET_MAG: u32 = 5;
const PACKET_RAW_CALIBRATION_DATA: u32 = 6;
const PACKET_CALIBRATION_FINISHED: u32 = 7;
const PACKET_CONFIG: u32 = 8;
const PACKET_RAW_MAGENTOMETER: u32 = 9;
const PACKET_PING_PONG: u32 = 10;
const PACKET_SERIAL: u32 = 11;
const PACKET_BATTERY_LEVEL: u32 = 12;
const PACKET_TAP: u32 = 13;
const PACKET_RESET_REASON: u32 = 14;
const PACKET_SENSOR_INFO: u32 = 15;
const PACKET_ROTATION_2: u32 = 16;
const PACKET_ROTATION_DATA: u32 = 17;
const PACKET_MAGENTOMETER_ACCURACY: u32 = 18;

const PACKET_BUTTON_PUSHED: u32 = 60;
const PACKET_SEND_MAG_STATUS: u32 = 61;
const PACKET_CHANGE_MAG_STATUS: u32 = 62;

const PACKET_RECIEVE_HEARTBEAT: u32 = 1;
const PACKET_RECIEVE_VIBRATE: u32 = 2;
const PACKET_RECIEVE_HANDSHAKE: u32 = 3;
const PACKET_RECIEVE_COMMAND: u32 = 4;

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

    let mac = tracker.properties().await.unwrap().unwrap().address;

    tracker.discover_services().await.unwrap();

    let chars = tracker.characteristics();

    let imu_data = chars
        .into_iter()
        .find(|c| c.uuid == uuid!("00dbf1c6-90aa-11ed-a1eb-0242ac120002"))
        .unwrap();

    let d = tracker.read(&imu_data).await.unwrap();

    // tracker.subscribe(&imu_data).await.unwrap();

    // let mut notifications = tracker.notifications().await.unwrap();

    // while let Some(data) = notifications.next().await {
    //     let mut cur = Cursor::new(data.value);
    //     struct Rotation {
    //         x: f32,
    //         y: f32,
    //         z: f32,
    //     }
    //     struct Accel {
    //         x: i16,
    //         y: i16,
    //         z: i16,
    //     }

    //     impl Debug for Rotation {
    //         fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    //             f.debug_struct("Rotation")
    //                 .field("x", &self.x)
    //                 .field("y", &self.y)
    //                 .field("z", &self.z)
    //                 .finish()
    //         }
    //     }

    //     let rotation = Rotation {
    //         x: cur.read_i16::<LittleEndian>().unwrap() as f32 / 100.0,
    //         y: cur.read_i16::<LittleEndian>().unwrap() as f32 / 100.0,
    //         z: cur.read_i16::<LittleEndian>().unwrap() as f32 / 100.0,
    //     };
    //     println!("Rotation: {:?}", rotation);
    //     // println!("IMU data: {:?}", cur.get_ref());
    // };

    // let target = "192.168.1.3:9186";
    let target = "255.255.255.255:6969";

    let mut rng = rand::thread_rng();

    let port = rng.gen_range(10000..20000);

    let socket = UdpSocket::bind(
        format!("127.0.0.1:{}", port)
            .parse::<std::net::SocketAddr>()
            .unwrap(),
    )
    .expect("Could not bind socket");

    socket.set_broadcast(true).expect("Could not set broadcast");
    socket
        .set_read_timeout(Some(std::time::Duration::from_millis(2500)))
        .unwrap();
    socket
        .set_write_timeout(Some(std::time::Duration::from_millis(2500)))
        .unwrap();

    let mut cur = Cursor::new([0 as u8; 12 + 36 + 9]); // 12 header, 36 slime info, 9 footer

    cur.write(PACKET_HANDSHAKE.to_be_bytes().as_ref()).unwrap(); // handshake packet
    cur.write(0u64.to_be_bytes().as_ref()).unwrap(); // handshake packet number

    let mac_bytes: [u8; 6] = mac.as_ref().try_into().unwrap(); // Convert the mac address to a byte array
    insert_slime_info(&mut cur, mac_bytes); // Pass the converted value

    println!("{:?}", cur.get_ref());

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
        println!("Packet: {}", std::str::from_utf8(buf).unwrap());
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

    let packet_count = Arc::new(Mutex::new(0u64));

    tracker.subscribe(&imu_data).await.unwrap();

    let mut notifications = tracker.notifications().await.unwrap();

    let sock = socket.try_clone().unwrap();
    let packet_c = packet_count.clone();
    tokio::spawn(async move {
        loop {
            let mut buf = Cursor::new([0u8; 12 + 4]); // 12 header, 4 battery level
            let _ = packet_c.lock().unwrap().wrapping_add(1);
            buf.write(PACKET_BATTERY_LEVEL.to_be_bytes().as_ref())
                .unwrap();
            buf.write(packet_c.lock().unwrap().to_be_bytes().as_ref())
                .unwrap();
            buf.write(0.33f32.to_be_bytes().as_ref()).unwrap();

            sock.send(buf.get_ref()).unwrap();

            time::sleep(std::time::Duration::from_millis(5000)).await;
        }
    });

    let sock = socket.try_clone().unwrap();
    let packet_c = packet_count.clone();
    tokio::spawn(async move {
        loop {
            let mut buf = [0u8; 256];
            sock.recv_from(buf.as_mut()).unwrap();
            parse_packet(&buf, &mut packet_c.lock().unwrap(), &sock);
        }
    });

    loop {
        let data = notifications.next().await.unwrap();

        let mut cur = Cursor::new(data.value);
        struct Rotation {
            x: f32,
            y: f32,
            z: f32,
            w: f32,
        }
        struct Gravity {
            x: f32,
            y: f32,
            z: f32,
        }

        impl Debug for Rotation {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Rotation")
                    .field("x", &self.x)
                    .field("y", &self.y)
                    .field("z", &self.z)
                    .field("w", &self.w)
                    .finish()
            }
        }
        impl Debug for Gravity {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.debug_struct("Gyro")
                    .field("x", &self.x)
                    .field("y", &self.y)
                    .field("z", &self.z)
                    .finish()
            }
        }

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

fn insert_slime_info(buf: &mut Cursor<[u8; 57]>, mac: [u8; 6]) {
    let board_type: u32 = 13;
    let imu_type: u32 = 0; // other
    let mcu_type: u32 = 3; // esp32

    // slimeVR version > 8 uses sensor info packet instead of imu info
    // it is kept here for backwards compatibility

    let imu_info: [u32; 3] = [0, 0, 0];

    let firmware_version_number: u32 = 8;

    let firmware_version = "HaritoraX-Wireless".as_bytes();

    let mac_address = mac;

    buf.write(board_type.to_be_bytes().as_ref()).unwrap();
    buf.write(imu_type.to_be_bytes().as_ref()).unwrap();
    buf.write(mcu_type.to_be_bytes().as_ref()).unwrap();

    for imu in imu_info.iter() {
        buf.write(imu.to_be_bytes().as_ref()).unwrap();
    }

    buf.write(firmware_version_number.to_be_bytes().as_ref())
        .unwrap();

    buf.write(firmware_version.len().to_be_bytes().as_ref())
        .unwrap();
    for byte in firmware_version.iter() {
        buf.write(byte.to_be_bytes().as_ref()).unwrap();
    }

    for byte in mac_address.iter() {
        buf.write(byte.to_be_bytes().as_ref()).unwrap();
    }
}

fn parse_packet(packet: &[u8], packet_count: &mut u64, socket: &UdpSocket) {
    let mut packet = Cursor::new(packet);

    let mut packet_type = [0u8; 4];
    packet.read_exact(packet_type.as_mut()).unwrap();

    let packet_type = u32::from_be_bytes(packet_type);

    match packet_type {
        PACKET_RECIEVE_HEARTBEAT => {
            println!("Received heartbeat");
        }
        PACKET_RECIEVE_VIBRATE => {
            println!("Received vibrate");
        }
        PACKET_CHANGE_MAG_STATUS => {
            println!("Received change mag status");
        }
        PACKET_PING_PONG => {
            println!("Received ping pong");
            socket.send(packet.get_ref()).unwrap();
        }
        _ => {
            println!("Received unknown packet type {}", packet_type);
        }
    }
}
