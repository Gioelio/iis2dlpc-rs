use defmt::info;
use maybe_async::maybe_async;
use crate::*;

#[maybe_async]
pub async fn run<B, D, L, I>(bus: B, mut tx: L, mut delay: D, mut int_pin: I) -> !
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
    // Set threshold to 60 degrees
    sensor.sixd_threshold_set(0x02).await.unwrap();
    // LPF2 on 6D function selection
    sensor.sixd_feed_data_set(LpassOn6d::Lpf2Feed).await.unwrap();
    // Enable interrupt generation on 6D INT1 pin
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_6d(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();
    // Set Output Data Rate
    sensor.data_rate_set(Odr::_200hz).await.unwrap();

    // Wait Events
    loop {
        int_pin.wait_for_event().await;

        // Check 6D Orientation events
        let all_sources = sensor.all_sources_get().await.unwrap();
        if all_sources.sixd_src.six_d_ia() == 1 {
            core::write!(tx, "6D or. switched to ").unwrap();

            if all_sources.sixd_src.xh() == 1 {
                core::write!(tx, "XH").unwrap();
            }

            if all_sources.sixd_src.xl() == 1 {
                core::write!(tx, "XL").unwrap();
            }

            if all_sources.sixd_src.yh() == 1 {
                core::write!(tx, "YH").unwrap();
            }

            if all_sources.sixd_src.yl() == 1 {
                core::write!(tx, "YL").unwrap();
            }

            if all_sources.sixd_src.zh() == 1 {
                core::write!(tx, "ZH").unwrap();
            }

            if all_sources.sixd_src.zl() == 1 {
                core::write!(tx, "ZL").unwrap();
            }

            writeln!(tx, "").unwrap();
        }
    }
}
