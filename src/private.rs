#![allow(unused)]

#[cfg(all(
    not(any(esp32s2, esp32p4)),
    esp_idf_bt_enabled,
    any(esp_idf_bt_bluedroid_enabled, feature = "trouble"),
))]
pub mod bt_controller;
pub mod common;
pub mod cstr;
pub mod mutex;
#[cfg(esp_idf_comp_esp_netif_enabled)]
pub mod net;
#[cfg(feature = "alloc")]
pub mod unblocker;
pub mod waitable;
#[cfg(feature = "alloc")]
pub mod zerocopy;

mod stubs;
