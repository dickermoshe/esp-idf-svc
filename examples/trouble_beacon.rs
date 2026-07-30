//! Non-connectable BLE beacon using the Trouble host with ESP-IDF's controller.
//!
//! Enable the `trouble` and `embassy-time-driver` Cargo features and use a
//! controller-only ESP-IDF configuration such as
//! `.github/configs/sdkconfig.defaults.trouble`.

#![allow(unknown_lints)]
#![allow(unexpected_cfgs)]

#[cfg(all(
    not(any(esp32s2, esp32p4)),
    feature = "trouble",
    esp_idf_bt_enabled,
    esp_idf_bt_controller_only,
))]
fn main() -> anyhow::Result<()> {
    example::main()
}

#[cfg(not(all(
    not(any(esp32s2, esp32p4)),
    feature = "trouble",
    esp_idf_bt_enabled,
    esp_idf_bt_controller_only,
)))]
fn main() -> anyhow::Result<()> {
    panic!(
        "This example requires the `trouble` feature and a controller-only configuration on a chip with a BLE radio"
    );
}

#[cfg(all(
    not(any(esp32s2, esp32p4)),
    feature = "trouble",
    esp_idf_bt_enabled,
    esp_idf_bt_controller_only,
))]
mod example {
    use anyhow::Result;
    use bt_hci::controller::ExternalController;
    use embassy_futures::select::{select, Either};
    use trouble_host::prelude::*;

    use esp_idf_svc::bt_controller::{Ble, EspBtController, EspVhciTransport};
    use esp_idf_svc::hal::peripherals::Peripherals;
    use esp_idf_svc::hal::task::block_on;
    use esp_idf_svc::nvs::EspDefaultNvsPartition;

    type Controller = ExternalController<EspVhciTransport<'static>, 10>;

    pub fn main() -> Result<()> {
        esp_idf_svc::sys::link_patches();
        let _ = esp_idf_svc::hal::task::critical_section::link();
        let _ = esp_idf_svc::timer::embassy_time_driver::link();
        esp_idf_svc::log::EspLogger::initialize_default();

        let peripherals = Peripherals::take()?;

        // Keep NVS alive for ESP-IDF's PHY calibration data.
        let _nvs = EspDefaultNvsPartition::take()?;

        let controller = EspBtController::<Ble>::new(peripherals.modem)?;
        let transport = EspVhciTransport::new(controller)?;
        let controller = ExternalController::<_, 10>::new(transport);

        block_on(run(controller));

        Ok(())
    }

    async fn run(controller: Controller) {
        let address = Address::random([0xff, 0x8f, 0x1a, 0x05, 0xe4, 0xff]);
        let mut resources: HostResources<Controller, DefaultPacketPool, 0, 0, 1> =
            HostResources::new();
        let stack = trouble_host::new(controller, &mut resources)
            .set_random_address(address)
            .build();
        let mut peripheral = stack.peripheral();
        let mut runner = stack.runner();

        let mut adv_data = [0; 31];
        let len = AdStructure::encode_slice(
            &[
                AdStructure::Flags(LE_GENERAL_DISCOVERABLE | BR_EDR_NOT_SUPPORTED),
                AdStructure::CompleteLocalName(b"ESP-IDF Trouble"),
            ],
            &mut adv_data,
        )
        .unwrap();

        log::info!("Starting Trouble beacon as {address:?}");

        match select(runner.run(), async {
            let _advertiser = peripheral
                .advertise(
                    &AdvertisementParameters::default(),
                    Advertisement::NonconnectableNonscannableUndirected {
                        adv_data: &adv_data[..len],
                    },
                )
                .await
                .unwrap();

            core::future::pending::<()>().await;
        })
        .await
        {
            Either::First(result) => result.unwrap(),
            Either::Second(()) => unreachable!("the advertising task never completes"),
        }
    }
}
