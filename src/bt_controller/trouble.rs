#[cfg(not(esp_idf_bt_controller_only))]
compile_error!("the `trouble` feature requires CONFIG_BT_CONTROLLER_ONLY=y");

#[cfg(all(
    any(esp32c2, esp32c5, esp32c6, esp32h2),
    not(esp_idf_bt_le_hci_interface_use_ram)
))]
compile_error!("the `trouble` feature requires CONFIG_BT_LE_HCI_INTERFACE_USE_RAM=y");

use core::cell::RefCell;
use core::fmt;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::collections::VecDeque;
use alloc::vec::Vec;

use bt_hci::transport::{Transport, WithIndicator};
use bt_hci::{
    ControllerToHostPacket, FromHciBytes, FromHciBytesError, HostToControllerPacket, WriteHci,
};
use embassy_sync::blocking_mutex::Mutex as BlockingMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use embedded_io::{ErrorKind, ErrorType};

use super::{Ble, EspBtController};
use crate::hal::interrupt::embassy_sync::IsrRawMutex;
use crate::hal::task::embassy_sync::EspRawMutex;
use crate::sys::{
    esp, esp_vhci_host_callback_t, esp_vhci_host_check_send_available,
    esp_vhci_host_register_callback, esp_vhci_host_send_packet, EspError, ESP_ERR_INVALID_STATE,
};

enum RxMessage {
    Packet(Vec<u8>),
    NullPacket,
}

struct RxState {
    queue: VecDeque<RxMessage>,
    error: Option<TransportError>,
}

static ACTIVE: AtomicBool = AtomicBool::new(false);
static RX_STATE: BlockingMutex<IsrRawMutex, RefCell<RxState>> =
    BlockingMutex::new(RefCell::new(RxState {
        queue: VecDeque::new(),
        error: None,
    }));
static RX_READY: Signal<IsrRawMutex, ()> = Signal::new();
static TX_READY: Signal<IsrRawMutex, ()> = Signal::new();

static VHCI_CALLBACKS: esp_vhci_host_callback_t = esp_vhci_host_callback_t {
    notify_host_send_available: Some(notify_host_send_available),
    notify_host_recv: Some(notify_host_recv),
};

unsafe extern "C" fn notify_host_send_available() {
    if ACTIVE.load(Ordering::SeqCst) {
        TX_READY.signal(());
    }
}

fn record_rx_error(error: TransportError) {
    RX_STATE.lock(|state| state.borrow_mut().error = Some(error));
    RX_READY.signal(());
}

fn enqueue_rx(message: RxMessage) {
    let len = match &message {
        RxMessage::Packet(bytes) => bytes.len(),
        RxMessage::NullPacket => 0,
    };

    RX_STATE.lock(|state| {
        let mut state = state.borrow_mut();

        if state.queue.try_reserve(1).is_err() {
            state.error = Some(TransportError::ReceiveAllocationFailed(len));
        } else {
            state.queue.push_back(message);
        }
    });

    // Wake the reader for either the queued packet or the allocation error
    // stored in `RX_STATE`.
    RX_READY.signal(());
}

unsafe extern "C" fn notify_host_recv(data: *mut u8, len: u16) -> core::ffi::c_int {
    if !ACTIVE.load(Ordering::SeqCst) {
        return 0;
    }

    let len = usize::from(len);
    let message = if data.is_null() {
        RxMessage::NullPacket
    } else {
        let mut bytes = Vec::new();

        if bytes.try_reserve_exact(len).is_err() {
            record_rx_error(TransportError::ReceiveAllocationFailed(len));
            return 0;
        }

        // SAFETY: ESP-IDF guarantees that `data` addresses `len` bytes for the
        // duration of this callback. The null case was checked above, and the
        // data is copied into owned storage before returning.
        bytes.extend_from_slice(unsafe { core::slice::from_raw_parts(data, len) });

        RxMessage::Packet(bytes)
    };

    enqueue_rx(message);

    0
}

/// Errors produced by the ESP-IDF VHCI transport.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransportError {
    /// The transport was used after its callback endpoint stopped.
    Inactive,
    /// Memory could not be allocated for a controller packet.
    ReceiveAllocationFailed(usize),
    /// ESP-IDF supplied a null packet pointer.
    NullReceivePacket,
    /// The buffer passed to `Transport::read` could not hold the packet.
    ReceiveBufferTooSmall {
        /// Packet length in bytes.
        required: usize,
        /// Supplied buffer length in bytes.
        available: usize,
    },
    /// A host packet exceeded the ESP-IDF VHCI API's `u16` length limit.
    TransmitPacketTooLarge(usize),
    /// Memory could not be allocated for a host packet.
    TransmitAllocationFailed(usize),
    /// An HCI packet was malformed.
    InvalidPacket(FromHciBytesError),
    /// Serialization into the transmit buffer unexpectedly failed.
    Serialize,
}

impl fmt::Display for TransportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Inactive => write!(f, "VHCI transport is inactive"),
            Self::ReceiveAllocationFailed(len) => {
                write!(f, "failed to allocate a {len}-byte controller packet")
            }
            Self::NullReceivePacket => write!(f, "ESP-IDF supplied a null packet pointer"),
            Self::ReceiveBufferTooSmall {
                required,
                available,
            } => write!(
                f,
                "receive buffer is too small: requires {required} bytes, but only {available} are available"
            ),
            Self::TransmitPacketTooLarge(len) => {
                write!(f, "{len}-byte host packet exceeds the VHCI length limit")
            }
            Self::TransmitAllocationFailed(len) => {
                write!(f, "failed to allocate a {len}-byte host packet")
            }
            Self::InvalidPacket(error) => write!(f, "invalid HCI packet: {error:?}"),
            Self::Serialize => write!(f, "failed to serialize the host packet"),
        }
    }
}

impl core::error::Error for TransportError {}

impl embedded_io::Error for TransportError {
    fn kind(&self) -> ErrorKind {
        match self {
            Self::Inactive => ErrorKind::NotConnected,
            Self::ReceiveAllocationFailed(_) | Self::TransmitAllocationFailed(_) => {
                ErrorKind::OutOfMemory
            }
            Self::NullReceivePacket | Self::InvalidPacket(_) => ErrorKind::InvalidData,
            Self::ReceiveBufferTooSmall { .. } => ErrorKind::OutOfMemory,
            Self::TransmitPacketTooLarge(_) => ErrorKind::InvalidInput,
            Self::Serialize => ErrorKind::WriteZero,
        }
    }
}

impl From<FromHciBytesError> for TransportError {
    fn from(value: FromHciBytesError) -> Self {
        Self::InvalidPacket(value)
    }
}

/// A packet-oriented `bt-hci` transport over ESP-IDF's virtual HCI interface.
///
/// The transport owns the controller so callbacks cannot outlive the
/// controller lifecycle. ESP-IDF exposes one global VHCI endpoint, hence only
/// one value can be active at a time.
pub struct EspVhciTransport<'d> {
    rx_lock: Mutex<EspRawMutex, ()>,
    tx_lock: Mutex<EspRawMutex, ()>,
    _controller: EspBtController<'d, Ble>,
}

impl<'d> EspVhciTransport<'d> {
    /// Registers the static VHCI callbacks and takes ownership of `controller`.
    pub fn new(controller: EspBtController<'d, Ble>) -> Result<Self, EspError> {
        if ACTIVE
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .is_err()
        {
            return Err(EspError::from_infallible::<ESP_ERR_INVALID_STATE>());
        }

        RX_STATE.lock(|state| {
            let mut state = state.borrow_mut();
            state.queue.clear();
            state.error = None;
        });
        RX_READY.reset();
        TX_READY.reset();

        if let Err(error) = esp!(unsafe { esp_vhci_host_register_callback(&VHCI_CALLBACKS) }) {
            ACTIVE.store(false, Ordering::SeqCst);
            return Err(error);
        }

        Ok(Self {
            rx_lock: Mutex::new(()),
            tx_lock: Mutex::new(()),
            _controller: controller,
        })
    }
}

impl ErrorType for EspVhciTransport<'_> {
    type Error = TransportError;
}

impl Transport for EspVhciTransport<'_> {
    async fn read<'a>(&self, rx: &'a mut [u8]) -> Result<ControllerToHostPacket<'a>, Self::Error> {
        if !ACTIVE.load(Ordering::SeqCst) {
            return Err(TransportError::Inactive);
        }

        let _guard = self.rx_lock.lock().await;

        let message = loop {
            let next = RX_STATE.lock(|state| {
                let mut state = state.borrow_mut();

                if let Some(error) = state.error.take() {
                    Err(error)
                } else {
                    Ok(state.queue.pop_front())
                }
            });

            match next? {
                Some(message) => break message,
                None => RX_READY.wait().await,
            }
        };

        let bytes = match message {
            RxMessage::Packet(bytes) => bytes,
            RxMessage::NullPacket => return Err(TransportError::NullReceivePacket),
        };
        let len = bytes.len();

        if rx.len() < len {
            return Err(TransportError::ReceiveBufferTooSmall {
                required: len,
                available: rx.len(),
            });
        }

        rx[..len].copy_from_slice(&bytes[..len]);
        ControllerToHostPacket::from_hci_bytes_complete(&rx[..len]).map_err(Into::into)
    }

    async fn write<T: HostToControllerPacket>(&self, value: &T) -> Result<(), Self::Error> {
        if !ACTIVE.load(Ordering::SeqCst) {
            return Err(TransportError::Inactive);
        }

        let _guard = self.tx_lock.lock().await;
        let packet = WithIndicator::new(value);
        let len = packet.size();
        let vhci_len =
            u16::try_from(len).map_err(|_| TransportError::TransmitPacketTooLarge(len))?;

        let mut bytes = Vec::new();
        bytes
            .try_reserve_exact(len)
            .map_err(|_| TransportError::TransmitAllocationFailed(len))?;
        bytes.resize(len, 0);
        let mut destination = bytes.as_mut_slice();
        packet
            .write_hci(&mut destination)
            .map_err(|_| TransportError::Serialize)?;

        loop {
            if !ACTIVE.load(Ordering::SeqCst) {
                return Err(TransportError::Inactive);
            }

            if unsafe { esp_vhci_host_check_send_available() } {
                break;
            }

            TX_READY.wait().await;
        }

        // ESP-IDF consumes the packet during this call. It requires a mutable
        // pointer even though the host does not modify the bytes.
        unsafe { esp_vhci_host_send_packet(bytes.as_mut_ptr(), vhci_len) };

        Ok(())
    }
}

impl Drop for EspVhciTransport<'_> {
    fn drop(&mut self) {
        ACTIVE.store(false, Ordering::SeqCst);
        RX_STATE.lock(|state| {
            let mut state = state.borrow_mut();
            state.queue.clear();
            state.error = None;
        });
        RX_READY.reset();
        TX_READY.reset();
    }
}
