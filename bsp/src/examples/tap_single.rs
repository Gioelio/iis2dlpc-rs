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
    // Configure power mode
    sensor
        .power_mode_set(Mode::ContLowPwrLowNoise1)
        .await.unwrap();
    // Set Output Data Rate
    sensor.data_rate_set(Odr::_400hz).await.unwrap();
    // Enable Tap detection on X, Y, Z
    sensor.tap_detection_on_z_set(PROPERTY_ENABLE).await.unwrap();
    sensor.tap_detection_on_y_set(PROPERTY_ENABLE).await.unwrap();
    sensor.tap_detection_on_x_set(PROPERTY_ENABLE).await.unwrap();
    // Set Tap threshold on all axis
    sensor.tap_threshold_x_set(9).await.unwrap();
    sensor.tap_threshold_y_set(9).await.unwrap();
    sensor.tap_threshold_z_set(9).await.unwrap();
    // Configure Single Tap parameter
    sensor.tap_quiet_set(1).await.unwrap();
    sensor.tap_shock_set(2).await.unwrap();
    // Enable single tap detection only
    sensor.tap_mode_set(SingleDoubleTap::OnlySingle).await.unwrap();
    // Enable single tap detection interrupt
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_single_tap(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();

    // Wait Events
    loop {
        int_pin.wait_for_event().await;

        // Check Single Tap events
        let all_sources = sensor.all_sources_get().await.unwrap();
        if all_sources.tap_src.single_tap() == 1 {
            core::write!(tx, "Tap Detected: Sign ").unwrap();

            if all_sources.tap_src.tap_sign() == 1 {
                core::write!(tx, "positive").unwrap();
            } else {
                core::write!(tx, "negative").unwrap();
            }

            core::write!(tx, " on ").unwrap();

            if all_sources.tap_src.x_tap() == 1 {
                core::write!(tx, "X ").unwrap();
            }

            if all_sources.tap_src.y_tap() == 1 {
                core::write!(tx, "Y ").unwrap();
            }

            if all_sources.tap_src.z_tap() == 1 {
                core::write!(tx, "Z ").unwrap();
            }

            core::writeln!(tx, "axis").unwrap();
        }
    }
}
