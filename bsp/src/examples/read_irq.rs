use defmt::info;
use maybe_async::maybe_async;
use crate::*;

#[maybe_async]
pub async fn run<B, D, L, I>(bus: B, mut tx: L, mut delay: D, mut int_pin : I) -> !
where
    B: BusOperation,
    D: DelayNs + Clone,
    L: embedded_io::Write,
    I: InterruptPin
{
    use iis2dlpc::prelude::*;
    use iis2dlpc::*;

    info!("Configuring the sensor");
    let mut sensor = Iis2dlpc::from_bus(bus, delay.clone());

    // boot time
    delay.delay_ms(25).await;

    // Check device ID
    let id = sensor.device_id_get().await.unwrap();
    info!("Device ID: {:x}", id);
    if id != ID {
        info!("Unexpected device ID: {:x}", id);
        writeln!(tx, "Unexpected device ID: {:x}", id).unwrap();
        loop {}
    }

    // Restore default configuration
    sensor.reset_set().await.unwrap();
    while sensor.reset_get().await.unwrap() == 1 {}

    // Enable Block Data Update
    sensor.block_data_update_set(PROPERTY_ENABLE).await.unwrap();
    // Set Full scale
    sensor.full_scale_set(Fs::_8g).await.unwrap();

    // Configure filtering chain

    // Accelerometer - filter path / bandwidth
    sensor.filter_path_set(Fds::LpfOnOut).await.unwrap();
    sensor.filter_bandwidth_set(BwFilt::OdrDiv4).await.unwrap();

    // Configure power mode
    sensor
        .power_mode_set(Mode::ContLowPwrLowNoise1)
        .await.unwrap();

    // Configure interrupt
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_drdy(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();

    // Set Output Data Rate
    sensor.data_rate_set(Odr::_25hz).await.unwrap();

    // Read samples in polling mode (no int)
    loop {
        int_pin.wait_for_event().await;

        if sensor.flag_data_ready_get().await.unwrap() == 1 {
            let acceleration_mg = sensor.acceleration_raw_get().await.unwrap().map(from_fs8_to_mg);

            writeln!(
                tx,
                "Acceleration [mg]: {:4.2}\t{:4.2}\t{:4.2}",
                acceleration_mg[0], acceleration_mg[1], acceleration_mg[2]
            )
            .unwrap();
        }
    }
}
