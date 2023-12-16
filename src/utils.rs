use std::{
    io::{Cursor, Read},
    net::UdpSocket,
};

use crate::constants::{
    constants::{
        PACKET_CHANGE_MAG_STATUS, PACKET_PING_PONG, PACKET_RECIEVE_HEARTBEAT,
        PACKET_RECIEVE_VIBRATE,
    },
    *,
};

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
	let mut hex_string = String::new();

	for byte in bytes {
		hex_string.push_str(&format!("{:02X}", byte));
	}

	hex_string
}

pub fn parse_packet(packet: &[u8], packet_count: &mut u64, socket: &UdpSocket) {
    let mut packet = Cursor::new(packet);

    let mut packet_type = [0u8; 4];
    packet.read_exact(packet_type.as_mut()).unwrap();

    let packet_type = u32::from_be_bytes(packet_type);

    match packet_type {
        PACKET_RECIEVE_HEARTBEAT => {
            // println!("Received heartbeat");
        }
        PACKET_RECIEVE_VIBRATE => {
            println!("Received vibrate");
        }
        PACKET_CHANGE_MAG_STATUS => {
            println!("Received change mag status");
        }
        PACKET_PING_PONG => {
            // println!("Received ping pong");
            socket.send(packet.get_ref()).unwrap();
        }
        _ => {
            println!("Received unknown packet type {}", packet_type);
        }
    }
}
