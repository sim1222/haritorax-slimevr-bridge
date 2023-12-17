use btleplug::{platform::{Adapter, Peripheral}, api::{Central as _, Peripheral as _}};

pub async fn find_tracker(central: &Adapter) -> Option<Peripheral> {
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

pub async fn find_trackers(central: &Adapter) -> Option<Vec<Peripheral>> {
	let mut peripherals = Vec::new();
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
			peripherals.push(peripheral);
		}
	}
	if peripherals.len() > 0 {
		Some(peripherals)
	} else {
		None
	}
}