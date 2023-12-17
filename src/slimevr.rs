use std::{
    error::Error,
    io::{Cursor, Read, Write},
};

use tokio::net::UdpSocket;

use crate::{
    constants::constants::{CURRENT_VERSION, PACKET_EOF, PACKET_HANDSHAKE},
    utils::bytes_to_hex_string,
};

pub fn insert_slime_info(buf: &mut Cursor<[u8; 66]>, mac: [u8; 6]) {
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

    buf.write(firmware_version_len.to_be_bytes().as_ref())
        .unwrap();

    buf.write(firmware_version.as_ref()).unwrap();

    buf.write(mac_address.as_ref()).unwrap();

    buf.write(PACKET_EOF.to_be_bytes().as_ref()).unwrap();
}

pub async fn try_handshake(
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

    socket.send_to(cur.get_ref(), target).await?; // Send the packet

    println!("Sent packet");

    let mut buf = [0u8; 64];

    let (_, src) = socket
        .recv_from(&mut buf)
        .await
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

    socket.connect(src).await.unwrap();

    println!("Connected to {}", src);

    Ok(())
}
