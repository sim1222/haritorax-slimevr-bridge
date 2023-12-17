use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
use rand::prelude::*;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time;
use uuid::Uuid;

mod bluetooth;
mod constants;
mod handler;
mod math;
mod slimevr;
mod utils;

use crate::bluetooth::{find_tracker, find_trackers};
use crate::constants::characteristics::{
    BATTERY_CHARACTERISTIC, MAIN_BUTTON_CHARACTERISTIC, SENSOR_CHARACTERISTIC,
};
use crate::handler::*;
use crate::slimevr::*;

use std::sync::atomic::AtomicU64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await.unwrap();
    let central = adapters.into_iter().nth(0).unwrap();

    central.start_scan(ScanFilter::default()).await.unwrap();

    println!("Scanning for 5 seconds...");

    time::sleep(std::time::Duration::from_millis(5000)).await;

    let trackers = find_trackers(&central)
        .await
        .expect("Could not find tracker");

    println!("Found {} trackers", trackers.len());

    for tracker in trackers {
        tokio::spawn(async move { tracker_worker(&tracker).await });
    }

    loop {
        time::sleep(std::time::Duration::from_millis(5000)).await;
    }
}

async fn tracker_worker(tracker: &Peripheral) {
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

    let port = rand::thread_rng().gen_range(10000..20000);

    let socket =
        UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], port)))
            .await
            .expect("Could not bind");

    // socket.set_broadcast(true).expect("Could not set broadcast");
    // socket
    //     .set_read_timeout(Some(std::time::Duration::from_millis(2500)))
    //     .unwrap();
    // socket
    //     .set_write_timeout(Some(std::time::Duration::from_millis(2500)))
    //     .unwrap();

    try_handshake(&socket, mac_bytes, target).await.unwrap();

    let packet_c = AtomicU64::new(0);
    let packet_c = &packet_c;

    tracker.subscribe(&imu_data).await.unwrap();
    tracker.subscribe(&battery_level).await.unwrap();
    tracker.subscribe(&main_button).await.unwrap();

    let mut notifications = tracker.notifications().await.unwrap();

    let mut buf = [0u8; 256];

    loop {
        tokio::select! {
            Ok(_) = socket.recv_from(buf.as_mut()) => {
                utils::parse_packet(&buf, packet_c, &socket).await;
            }
            Some(data) = notifications.next() => {
                match data
                    .uuid
                    .hyphenated()
                    .encode_lower(&mut Uuid::encode_buffer())
                {
                    uuid if uuid == SENSOR_CHARACTERISTIC.to_owned().as_str() => {
                        handle_imu_data(&data.value, &socket, &packet_c).await;
                    }
                    uuid if uuid == BATTERY_CHARACTERISTIC.to_owned().as_str() => {
                        handle_battery_data(&data.value, &socket, &packet_c).await;
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
    }
}
