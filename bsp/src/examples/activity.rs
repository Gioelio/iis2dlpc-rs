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

    // Set Full scale
    sensor.full_scale_set(Fs::_2g).await.unwrap();
    // Configure filtering chain
    // Accelerometer - filter path / bandwidth
    sensor.filter_path_set(Fds::LpfOnOut).await.unwrap();
    sensor.filter_bandwidth_set(BwFilt::OdrDiv4).await.unwrap();
    // Configure power mode
    sensor
        .power_mode_set(Mode::ContLowPwrLowNoise1)
        .await.unwrap();
    // Set wake-up duration
    // Wake up duration event 1Lsb = 1 / ODR
    sensor.wkup_dur_set(2).await.unwrap();
    // Set sleep duration
    // Duration to go in sleep mode (1 = Lsb 512 / ODR)
    sensor.act_sleep_dur_set(2).await.unwrap();
    // Set Activity wake-up threshold
    // Threshold for wake-up 1 LSB = FS_XL / 64
    sensor.wkup_threshold_set(2).await.unwrap();
    // Data sent to wake-up interrupt function
    sensor.wkup_feed_data_set(UsrOffOnWu::HpFeed).await.unwrap();
    // Config activity / inactivity of stationary / motion detection
    sensor.act_mode_set(SleepOn::ActInact).await.unwrap();
    // Enable activiy detection interrupt
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_wu(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();
    // Enable inactivity detection interrupt on int2 and redirect to int1
    let mut int2_route = sensor.pin_int2_route_get().await.unwrap();
    int2_route.set_int2_sleep_chg(1);
    sensor.pin_int2_route_set(&int2_route).await.unwrap();
    sensor.all_on_int1_set(1).await.unwrap();
    // Set Output Data Rate
    sensor.data_rate_set(Odr::_200hz).await.unwrap();

    // Wait Events
    loop {
        int_pin.wait_for_event().await;

        // Read status register
        let all_sources = sensor.all_sources_get().await.unwrap();

        // Check if Activity/Inactivity events
        if all_sources.wake_up_src.sleep_state_ia() == 1 {
            writeln!(tx, "Inactivity Detected").unwrap();
        }

        if all_sources.wake_up_src.wu_ia() == 1 {
            writeln!(tx, "Activity Detected").unwrap();
        }
    }
}
