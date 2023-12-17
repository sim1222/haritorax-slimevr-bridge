use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
use rand::prelude::*;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time;

mod bluetooth;
mod constants;
mod handler;
mod haritora;
mod math;
mod slimevr;
mod utils;

use crate::bluetooth::{find_tracker, find_trackers};
use crate::handler::*;
use crate::slimevr::*;

use std::sync::atomic::AtomicU64;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let manager = Manager::new().await.unwrap();
    let adapters = manager.adapters().await.unwrap();
    let central = adapters.first().unwrap();

    central.start_scan(ScanFilter::default()).await.unwrap();

    println!("Scanning for 5 seconds...");

    time::sleep(std::time::Duration::from_millis(5000)).await;

    let trackers = find_trackers(central).await;

    if trackers.is_empty() {
        panic!("Could not find tracker");
    }

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
        .iter()
        .find(|c| c.uuid == haritora::Characteristics::Sensor.into())
        .unwrap();

    let battery_level = chars
        .iter()
        .find(|c| c.uuid == haritora::Characteristics::Battery.into())
        .unwrap();

    let main_button = chars
        .iter()
        .find(|c| c.uuid == haritora::Characteristics::MainButton.into())
        .unwrap();

    let target = "192.168.1.3:6969";

    let port = rand::thread_rng().gen_range(10000..20000);

    let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("Could not bind");

    try_handshake(&socket, mac_bytes, target).await.unwrap();

    let packet_c = AtomicU64::new(0);

    tracker.subscribe(&imu_data).await.unwrap();
    tracker.subscribe(&battery_level).await.unwrap();
    tracker.subscribe(&main_button).await.unwrap();

    let mut notifications = tracker.notifications().await.unwrap();
    let mut buf = [0u8; 256];

    loop {
        tokio::select! {
            Ok(_) = socket.recv_from(buf.as_mut()) => {
                utils::parse_packet(&buf, &packet_c, &socket).await;
            }
            Some(data) = notifications.next() => {
                match data.uuid {
                    uuid if uuid == haritora::Characteristics::Sensor.into() => {
                        let (rotation, gravity) = haritora::decode_imu_packet(&data.value).unwrap();
                        handle_imu_data(rotation, gravity, &socket, &packet_c).await;
                    }
                    uuid if uuid == haritora::Characteristics::Battery.into() => {
                        let battery_level = haritora::decode_battery_packet(&data.value).unwrap();
                        handle_battery_data(battery_level, &socket, &packet_c).await;
                    }
                    uuid if uuid == haritora::Characteristics::MainButton.into() => {
                        println!("Received button push");
                    }
                    _ => unreachable!("BLE connection maybe corrupted"),
                }
            }
        }
    }
}
