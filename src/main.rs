use btleplug::api::{Central, Manager as _, Peripheral as _, ScanFilter};
use btleplug::platform::{Manager, Peripheral};
use futures::stream::StreamExt;
use rand::prelude::*;
use std::error::Error;
use std::net::SocketAddr;
use tokio::net::UdpSocket;
use tokio::time;

mod manager;
mod haritora;
mod math;
mod slimevr;

use crate::manager::find_trackers;

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


    let port = rand::thread_rng().gen_range(10000..20000);

    let socket = UdpSocket::bind(SocketAddr::from(([0, 0, 0, 0], port)))
        .await
        .expect("Could not bind");

    let b = slimevr::BoardInfo::new(&mac_bytes).firmware_version("HaritoraX-Wireless");
    let mut slime_client = slimevr::Client::try_new(socket, &b).await.unwrap();

    tracker.subscribe(&imu_data).await.unwrap();
    tracker.subscribe(&battery_level).await.unwrap();
    tracker.subscribe(&main_button).await.unwrap();

    let mut notifications = tracker.notifications().await.unwrap();

    loop {
        tokio::select! {
            _ = slime_client.recv() => {
                // do nothing
            },

            Some(data) = notifications.next() => {
                match data.uuid {
                    uuid if uuid == haritora::Characteristics::Sensor.into() => {
                        let (rotation, gravity) = haritora::decode_imu_packet(&data.value).unwrap();
                        slime_client.try_send_rotation(&rotation).await.unwrap();
                        slime_client.try_send_gravity(&gravity).await.unwrap();
                    }
                    uuid if uuid == haritora::Characteristics::Battery.into() => {
                        let battery_level = haritora::decode_battery_packet(&data.value).unwrap();
                        slime_client.try_send_battery_level(battery_level).await.unwrap();
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
