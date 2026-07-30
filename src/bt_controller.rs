//! Bare ESP-IDF Bluetooth controller support.
//!
//! Enable the `trouble` Cargo feature to use the controller with the
//! [Trouble](https://github.com/embassy-rs/trouble) host through `bt-hci`:
//!
//! ```toml
//! esp-idf-svc = { version = "0.52", features = ["trouble"] }
//! ```
//!
//! The application must select the controller-only RAM HCI interface in
//! `sdkconfig.defaults`:
//!
//! ```text
//! CONFIG_BT_ENABLED=y
//! CONFIG_BT_CONTROLLER_ONLY=y
//! CONFIG_BT_CONTROLLER_ENABLED=y
//! CONFIG_BT_LE_HCI_INTERFACE_USE_RAM=y
//! ```
use core::marker::PhantomData;

use crate::hal::modem::BluetoothModemPeripheral;
use crate::sys::*;

pub trait BtMode: Send {
    fn mode() -> esp_bt_mode_t;
}

pub trait BleEnabled: BtMode {}
pub trait BtClassicEnabled: BtMode {}

#[cfg(esp32)]
#[cfg(not(esp_idf_btdm_ctrl_mode_ble_only))]
#[derive(Clone)]
pub struct BtClassic(());
#[cfg(esp32)]
#[cfg(not(esp_idf_btdm_ctrl_mode_ble_only))]
impl BtClassicEnabled for BtClassic {}

#[cfg(esp32)]
#[cfg(not(esp_idf_btdm_ctrl_mode_ble_only))]
impl BtMode for BtClassic {
    fn mode() -> esp_bt_mode_t {
        #[cfg(not(esp_idf_btdm_ctrl_mode_btdm))]
        let mode = esp_bt_mode_t_ESP_BT_MODE_CLASSIC_BT;

        #[cfg(esp_idf_btdm_ctrl_mode_btdm)]
        let mode = esp_bt_mode_t_ESP_BT_MODE_BTDM;

        mode
    }
}

#[cfg(not(esp_idf_btdm_ctrl_mode_br_edr_only))]
#[derive(Clone)]
pub struct Ble(());
#[cfg(not(esp_idf_btdm_ctrl_mode_br_edr_only))]
impl BleEnabled for Ble {}

#[cfg(not(esp_idf_btdm_ctrl_mode_br_edr_only))]
impl BtMode for Ble {
    fn mode() -> esp_bt_mode_t {
        #[cfg(not(esp_idf_btdm_ctrl_mode_btdm))]
        let mode = esp_bt_mode_t_ESP_BT_MODE_BLE;

        #[cfg(esp_idf_btdm_ctrl_mode_btdm)]
        let mode = esp_bt_mode_t_ESP_BT_MODE_BTDM;

        mode
    }
}

#[cfg(esp32)]
#[cfg(esp_idf_btdm_ctrl_mode_btdm)]
#[derive(Clone)]
pub struct BtDual(());
#[cfg(esp32)]
#[cfg(esp_idf_btdm_ctrl_mode_btdm)]
impl BtClassicEnabled for BtDual {}
#[cfg(esp32)]
#[cfg(esp_idf_btdm_ctrl_mode_btdm)]
impl BleEnabled for BtDual {}

#[cfg(esp32)]
#[cfg(esp_idf_btdm_ctrl_mode_btdm)]
impl BtMode for BtDual {
    fn mode() -> esp_bt_mode_t {
        esp_bt_mode_t_ESP_BT_MODE_BTDM
    }
}

/// An initialized and enabled ESP-IDF Bluetooth controller without a host.
///
/// This type consumes the HAL Bluetooth modem token. Dropping it disables and
/// deinitializes the controller.
pub struct EspBtController<'d, M>
where
    M: BtMode,
{
    _p: PhantomData<&'d mut ()>,
    _m: PhantomData<M>,
}

impl<'d, M> EspBtController<'d, M>
where
    M: BtMode,
{
    /// Initializes and enables the controller in `M`'s Bluetooth mode.
    pub fn new<B>(_modem: B) -> Result<Self, EspError>
    where
        B: BluetoothModemPeripheral + 'd,
    {
        crate::private::bt_controller::init(M::mode(), false)?;

        Ok(Self {
            _p: PhantomData,
            _m: PhantomData,
        })
    }
}

impl<M> Drop for EspBtController<'_, M>
where
    M: BtMode,
{
    fn drop(&mut self) {
        esp!(unsafe { esp_bt_controller_disable() }).unwrap();

        esp!(unsafe { esp_bt_controller_deinit() }).unwrap();
    }
}

#[cfg(feature = "trouble")]
mod trouble;

#[cfg(feature = "trouble")]
pub use trouble::{EspVhciTransport, TransportError};
