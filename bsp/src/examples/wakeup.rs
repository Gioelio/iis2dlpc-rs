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
        .await
        .unwrap();
    // Set Output Data Rate
    sensor.data_rate_set(Odr::_200hz).await.unwrap();
    // Apply hogh-pass digital filter on Wake-Up function
    // Duration time is set to zero so Wake-Up interrupt signal
    // is generated for each X,Y,Z filtered data exceeding the
    // configured threshold
    sensor.wkup_dur_set(0).await.unwrap();
    // Set wake-up threshold
    // Set wake-up threshold: 1 Lsb corresponds to FS_XL/2^6
    sensor.wkup_threshold_set(2).await.unwrap();
    // Enable interrupt generation on Wake-Up INT1 pin
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_wu(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();

    // Wait Events
    loop {
        int_pin.wait_for_event().await;

        // Check Wake-Up events
        let all_sources = sensor.all_sources_get().await.unwrap();
        if all_sources.wake_up_src.wu_ia() == 1 {
            core::write!(tx, "Wake-Up event on ").unwrap();

            if all_sources.wake_up_src.x_wu() == 1 {
                core::write!(tx, "X").unwrap();
            }

            if all_sources.wake_up_src.y_wu() == 1 {
                core::write!(tx, "Y").unwrap();
            }

            if all_sources.wake_up_src.z_wu() == 1 {
                core::write!(tx, "Z").unwrap();
            }

            core::writeln!(tx, " direction.").unwrap();
        }
    }
}
