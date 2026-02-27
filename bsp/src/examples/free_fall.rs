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

    // Configure power mode
    sensor
        .power_mode_set(Mode::HighPerformanceLowNoise)
        .await.unwrap();
    // Set Output Data Rate
    sensor.data_rate_set(Odr::_200hz).await.unwrap();
    // Set full scale to 2g
    sensor.full_scale_set(Fs::_2g).await.unwrap();
    // Configure Free Fall duration and samples count
    sensor.ff_dur_set(0x06).await.unwrap();
    sensor.ff_threshold_set(FfThs::_10Lsb).await.unwrap();
    // nable free fall interrupt
    let mut int_route = sensor.pin_int1_route_get().await.unwrap();
    int_route.set_int1_ff(PROPERTY_ENABLE);
    sensor.pin_int1_route_set(&int_route).await.unwrap();
    // Set latched interrupt
    sensor.int_notification_set(Lir::Latched).await.unwrap();

    // Wait Events
    loop {
        int_pin.wait_for_event().await;

        // Check Free Fall events
        let all_sources = sensor.all_sources_get().await.unwrap();
        if all_sources.wake_up_src.ff_ia() == 1 {
            writeln!(tx, "Free fall detected").unwrap();
        }
    }
}
