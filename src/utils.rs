use std::io::{Cursor, Read};

use tokio::net::UdpSocket;

use crate::constants::constants::RxPacketType;

pub fn bytes_to_hex_string(bytes: &[u8]) -> String {
    let mut hex_string = String::new();

    for byte in bytes {
        hex_string.push_str(&format!("{:02X}", byte));
    }

    hex_string
}

pub async fn parse_packet(packet: &[u8], packet_count: &mut u64, socket: &UdpSocket) {
    let mut packet = Cursor::new(packet);

    let mut packet_type = [0u8; 4];
    packet.read_exact(packet_type.as_mut()).unwrap();

    let packet_type = RxPacketType::try_from(u32::from_be_bytes(packet_type));

    match packet_type {
        Ok(RxPacketType::Heartbeat) => println!("Received heartbeat"),
        Ok(RxPacketType::Vibrate) => println!("Received vibrate"),
        Ok(RxPacketType::PingPong) => {
            socket.send(packet.get_ref()).await.unwrap();
            println!("Received ping pong");
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
