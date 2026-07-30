use core::cell::UnsafeCell;
use core::fmt::{self, Debug};
use core::marker::PhantomData;
use core::sync::atomic::{AtomicBool, Ordering};

use alloc::boxed::Box;
use alloc::sync::Arc;

use ::log::info;

use num_enum::TryFromPrimitive;

use crate::hal::modem::BluetoothModemPeripheral;

use crate::private::mutex::{self, Mutex};
use crate::sys::*;

#[cfg(all(feature = "alloc", esp_idf_comp_nvs_flash_enabled))]
use crate::nvs::EspDefaultNvsPartition;

extern crate alloc;

#[cfg(all(esp32, esp_idf_bt_classic_enabled, esp_idf_bt_a2dp_enable))]
pub mod a2dp;
#[cfg(all(esp32, esp_idf_bt_classic_enabled, esp_idf_bt_a2dp_enable))]
pub mod avrc;
pub mod ble;
#[cfg(all(esp32, esp_idf_bt_classic_enabled))]
pub mod gap;
#[cfg(all(esp32, esp_idf_bt_classic_enabled, esp_idf_bt_hfp_enable))]
pub mod hfp;
#[cfg(all(esp32, esp_idf_bt_classic_enabled, esp_idf_bt_spp_enabled))]
pub mod spp;

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
#[repr(transparent)]
pub struct BdAddr(esp_bd_addr_t);

impl BdAddr {
    pub const fn raw(&self) -> &esp_bd_addr_t {
        &self.0
    }

    pub const fn from_bytes(bytes: [u8; 6]) -> Self {
        Self(bytes)
    }

    pub const fn addr(&self) -> [u8; 6] {
        self.0
    }
}

impl fmt::Display for BdAddr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X}",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

impl From<BdAddr> for esp_bd_addr_t {
    fn from(value: BdAddr) -> Self {
        value.0
    }
}

impl From<esp_bd_addr_t> for BdAddr {
    fn from(value: esp_bd_addr_t) -> Self {
        Self(value)
    }
}

#[derive(Clone)]
#[repr(transparent)]
pub struct BtUuid(esp_bt_uuid_t);

impl BtUuid {
    pub const fn raw(&self) -> &esp_bt_uuid_t {
        &self.0
    }

    pub const fn into_raw(self) -> esp_bt_uuid_t {
        self.0
    }

    pub const fn uuid16(uuid: u16) -> Self {
        let esp_uuid = esp_bt_uuid_t {
            len: 2,
            uuid: esp_bt_uuid_t__bindgen_ty_1 { uuid16: uuid },
        };

        Self(esp_uuid)
    }

    pub const fn uuid32(uuid: u32) -> Self {
        let esp_uuid = esp_bt_uuid_t {
            len: 4,
            uuid: esp_bt_uuid_t__bindgen_ty_1 { uuid32: uuid },
        };

        Self(esp_uuid)
    }

    pub const fn uuid128(uuid: u128) -> Self {
        let esp_uuid = esp_bt_uuid_t {
            len: 16,
            uuid: esp_bt_uuid_t__bindgen_ty_1 {
                uuid128: uuid.to_le_bytes(),
            },
        };

        Self(esp_uuid)
    }

    pub const fn as_bytes(&self) -> &[u8] {
        match self.0.len {
            2 => unsafe {
                core::slice::from_raw_parts(&self.0.uuid.uuid128 as *const _ as *const _, 2)
            },
            4 => unsafe {
                core::slice::from_raw_parts(&self.0.uuid.uuid128 as *const _ as *const _, 4)
            },
            16 => unsafe { &self.0.uuid.uuid128 },
            _ => unreachable!(),
        }
    }
}

impl Debug for BtUuid {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "BtUuid {{{:?}}}", self.as_bytes())
    }
}

impl PartialEq for BtUuid {
    fn eq(&self, other: &BtUuid) -> bool {
        self.as_bytes() == other.as_bytes()
    }
}

impl Eq for BtUuid {}

impl From<BtUuid> for esp_bt_uuid_t {
    fn from(uuid: BtUuid) -> Self {
        uuid.0
    }
}

impl From<esp_bt_uuid_t> for BtUuid {
    fn from(uuid: esp_bt_uuid_t) -> Self {
        Self(uuid)
    }
}

#[allow(dead_code)]
#[allow(clippy::type_complexity)]
pub(crate) struct BtSingleton<A, R> {
    initialized: AtomicBool,
    callback: Mutex<Option<Arc<UnsafeCell<Box<dyn FnMut(A) -> R>>>>>,
    default_result: R,
}

#[allow(dead_code)]
impl<A, R> BtSingleton<A, R>
where
    R: Clone,
{
    pub const fn new(default_result: R) -> Self {
        Self {
            initialized: AtomicBool::new(false),
            callback: Mutex::new(None),
            default_result,
        }
    }

    pub fn release(&self) -> Result<(), EspError> {
        self.unsubscribe();

        self.initialized
            .compare_exchange(true, false, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| EspError::from_infallible::<ESP_ERR_INVALID_STATE>())?;

        Ok(())
    }

    pub fn take(&self) -> Result<(), EspError> {
        self.initialized
            .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
            .map_err(|_| EspError::from_infallible::<ESP_ERR_INVALID_STATE>())?;

        Ok(())
    }

    pub fn subscribe<'d, F>(&self, callback: F)
    where
        F: FnMut(A) -> R + Send + 'd,
    {
        let callback = unsafe {
            core::mem::transmute::<
                Box<dyn FnMut(A) -> R + Send + 'd>,
                Box<dyn FnMut(A) -> R + Send + 'static>,
            >(Box::new(callback))
        };

        *self.callback.lock() = Some(Arc::new(UnsafeCell::new(callback)));
    }

    pub fn unsubscribe(&self) {
        *self.callback.lock() = None;
    }

    /// Safe to use only from within the ESP IDF Bluedroid task
    pub unsafe fn call(&self, arg: A) -> R {
        if let Some(callback) = self
            .callback
            .lock()
            .as_ref()
            .map(|callback| callback.clone())
        {
            ((callback.get()).as_mut().unwrap())(arg)
        } else {
            self.default_result.clone()
        }
    }
}

unsafe impl<A, R> Sync for BtSingleton<A, R> {}
unsafe impl<A, R> Send for BtSingleton<A, R> {}

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

#[derive(Debug, Copy, Clone, Eq, PartialEq, TryFromPrimitive)]
#[repr(u32)]
pub enum BtStatus {
    Success = esp_bt_status_t_ESP_BT_STATUS_SUCCESS,
    Fail = esp_bt_status_t_ESP_BT_STATUS_FAIL,
    NotReady = esp_bt_status_t_ESP_BT_STATUS_NOT_READY,
    NoMem = esp_bt_status_t_ESP_BT_STATUS_NOMEM,
    Busy = esp_bt_status_t_ESP_BT_STATUS_BUSY,
    Done = esp_bt_status_t_ESP_BT_STATUS_DONE,
    Unsupported = esp_bt_status_t_ESP_BT_STATUS_UNSUPPORTED,
    InvalidParam = esp_bt_status_t_ESP_BT_STATUS_PARM_INVALID,
    Unhandled = esp_bt_status_t_ESP_BT_STATUS_UNHANDLED,
    AuthFailure = esp_bt_status_t_ESP_BT_STATUS_AUTH_FAILURE,
    RemoteDeviceDown = esp_bt_status_t_ESP_BT_STATUS_RMT_DEV_DOWN,
    AuthRejected = esp_bt_status_t_ESP_BT_STATUS_AUTH_REJECTED,
    InvalidStaticRandAddr = esp_bt_status_t_ESP_BT_STATUS_INVALID_STATIC_RAND_ADDR,
    Pending = esp_bt_status_t_ESP_BT_STATUS_PENDING,
    UnacceptedConnInterval = esp_bt_status_t_ESP_BT_STATUS_UNACCEPT_CONN_INTERVAL,
    ParamOutOfRange = esp_bt_status_t_ESP_BT_STATUS_PARAM_OUT_OF_RANGE,
    Timeout = esp_bt_status_t_ESP_BT_STATUS_TIMEOUT,
    UnsupportedPeerLeDataLen = esp_bt_status_t_ESP_BT_STATUS_PEER_LE_DATA_LEN_UNSUPPORTED,
    UnsupportedControlLeDataLen = esp_bt_status_t_ESP_BT_STATUS_CONTROL_LE_DATA_LEN_UNSUPPORTED,
    IllegalParamFormat = esp_bt_status_t_ESP_BT_STATUS_ERR_ILLEGAL_PARAMETER_FMT,
    MemoryFull = esp_bt_status_t_ESP_BT_STATUS_MEMORY_FULL,
    EirTooLarge = esp_bt_status_t_ESP_BT_STATUS_EIR_TOO_LARGE,
    HciSuccess = esp_bt_status_t_ESP_BT_STATUS_HCI_SUCCESS,
    HciIllegalCommand = esp_bt_status_t_ESP_BT_STATUS_HCI_ILLEGAL_COMMAND,
    HciNoConnection = esp_bt_status_t_ESP_BT_STATUS_HCI_NO_CONNECTION,
    HciHwFailure = esp_bt_status_t_ESP_BT_STATUS_HCI_HW_FAILURE,
    HciPageTimeout = esp_bt_status_t_ESP_BT_STATUS_HCI_PAGE_TIMEOUT,
    HciAuthFailure = esp_bt_status_t_ESP_BT_STATUS_HCI_AUTH_FAILURE,
    HciKeyMissing = esp_bt_status_t_ESP_BT_STATUS_HCI_KEY_MISSING,
    HciMemoryFull = esp_bt_status_t_ESP_BT_STATUS_HCI_MEMORY_FULL,
    HciConnTimeout = esp_bt_status_t_ESP_BT_STATUS_HCI_CONNECTION_TOUT,
    HciConnectionsExhausted = esp_bt_status_t_ESP_BT_STATUS_HCI_MAX_NUM_OF_CONNECTIONS,
    HciScosExhausted = esp_bt_status_t_ESP_BT_STATUS_HCI_MAX_NUM_OF_SCOS,
    HciConnectionExists = esp_bt_status_t_ESP_BT_STATUS_HCI_CONNECTION_EXISTS,
    HciCommandDisallowed = esp_bt_status_t_ESP_BT_STATUS_HCI_COMMAND_DISALLOWED,
    HciHostResourcesRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_HOST_REJECT_RESOURCES,
    HciHostSecurityRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_HOST_REJECT_SECURITY,
    HciHostDevideRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_HOST_REJECT_DEVICE,
    HciHostTimeout = esp_bt_status_t_ESP_BT_STATUS_HCI_HOST_TIMEOUT,
    HciUnsupportedValue = esp_bt_status_t_ESP_BT_STATUS_HCI_UNSUPPORTED_VALUE,
    HciIllegalParamFormat = esp_bt_status_t_ESP_BT_STATUS_HCI_ILLEGAL_PARAMETER_FMT,
    HciPeerUser = esp_bt_status_t_ESP_BT_STATUS_HCI_PEER_USER,
    HciPeerLowResources = esp_bt_status_t_ESP_BT_STATUS_HCI_PEER_LOW_RESOURCES,
    HciPeerPowerOff = esp_bt_status_t_ESP_BT_STATUS_HCI_PEER_POWER_OFF,
    HciConnectionCauseLocalHost = esp_bt_status_t_ESP_BT_STATUS_HCI_CONN_CAUSE_LOCAL_HOST,
    HciRepeatedAttempts = esp_bt_status_t_ESP_BT_STATUS_HCI_REPEATED_ATTEMPTS,
    HciPairingNotAllowed = esp_bt_status_t_ESP_BT_STATUS_HCI_PAIRING_NOT_ALLOWED,
    HciUnkownLmpPdu = esp_bt_status_t_ESP_BT_STATUS_HCI_UNKNOWN_LMP_PDU,
    HciUnsupportedRemFeature = esp_bt_status_t_ESP_BT_STATUS_HCI_UNSUPPORTED_REM_FEATURE,
    HciScoOffsetRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_SCO_OFFSET_REJECTED,
    HciScoInternalRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_SCO_INTERVAL_REJECTED,
    HciScoAirMode = esp_bt_status_t_ESP_BT_STATUS_HCI_SCO_AIR_MODE,
    HciInvalidLmpParam = esp_bt_status_t_ESP_BT_STATUS_HCI_INVALID_LMP_PARAM,
    HciUnspecified = esp_bt_status_t_ESP_BT_STATUS_HCI_UNSPECIFIED,
    HciUnsupportedLmpParameters = esp_bt_status_t_ESP_BT_STATUS_HCI_UNSUPPORTED_LMP_PARAMETERS,
    HciRoleChangeNotAllowed = esp_bt_status_t_ESP_BT_STATUS_HCI_ROLE_CHANGE_NOT_ALLOWED,
    HciLmpResponseTimeout = esp_bt_status_t_ESP_BT_STATUS_HCI_LMP_RESPONSE_TIMEOUT,
    HciLmpErrTransactionCollision = esp_bt_status_t_ESP_BT_STATUS_HCI_LMP_ERR_TRANS_COLLISION,
    HciLmpPduNotAllowed = esp_bt_status_t_ESP_BT_STATUS_HCI_LMP_PDU_NOT_ALLOWED,
    HciEntryModeNotAcceptable = esp_bt_status_t_ESP_BT_STATUS_HCI_ENCRY_MODE_NOT_ACCEPTABLE,
    HciUnitKeyUsed = esp_bt_status_t_ESP_BT_STATUS_HCI_UNIT_KEY_USED,
    HciUnsupportedQos = esp_bt_status_t_ESP_BT_STATUS_HCI_QOS_NOT_SUPPORTED,
    HciInstantPassed = esp_bt_status_t_ESP_BT_STATUS_HCI_INSTANT_PASSED,
    HciUnsupportedPairingWithUnitKey =
        esp_bt_status_t_ESP_BT_STATUS_HCI_PAIRING_WITH_UNIT_KEY_NOT_SUPPORTED,
    HciDiffTransactionCollision = esp_bt_status_t_ESP_BT_STATUS_HCI_DIFF_TRANSACTION_COLLISION,
    HciUndefined0x2b = esp_bt_status_t_ESP_BT_STATUS_HCI_UNDEFINED_0x2B,
    HciQosInvalidParam = esp_bt_status_t_ESP_BT_STATUS_HCI_QOS_UNACCEPTABLE_PARAM,
    HciQosRejected = esp_bt_status_t_ESP_BT_STATUS_HCI_QOS_REJECTED,
    HciUnsupportedChanClassification = esp_bt_status_t_ESP_BT_STATUS_HCI_CHAN_CLASSIF_NOT_SUPPORTED,
    HciInsufficientSecurity = esp_bt_status_t_ESP_BT_STATUS_HCI_INSUFFCIENT_SECURITY,
    HciParamOutOfRange = esp_bt_status_t_ESP_BT_STATUS_HCI_PARAM_OUT_OF_RANGE,
    HciUndefined0x31 = esp_bt_status_t_ESP_BT_STATUS_HCI_UNDEFINED_0x31,
    HciRoleSwitchPending = esp_bt_status_t_ESP_BT_STATUS_HCI_ROLE_SWITCH_PENDING,
    HciUndefined0x33 = esp_bt_status_t_ESP_BT_STATUS_HCI_UNDEFINED_0x33,
    HciReservedSlotViolation = esp_bt_status_t_ESP_BT_STATUS_HCI_RESERVED_SLOT_VIOLATION,
    HciRoleSwitchFailed = esp_bt_status_t_ESP_BT_STATUS_HCI_ROLE_SWITCH_FAILED,
    HciInqRespDataTooLarge = esp_bt_status_t_ESP_BT_STATUS_HCI_INQ_RSP_DATA_TOO_LARGE,
    HciSimplePairingNotSupported = esp_bt_status_t_ESP_BT_STATUS_HCI_SIMPLE_PAIRING_NOT_SUPPORTED,
    HciHostBusyPairing = esp_bt_status_t_ESP_BT_STATUS_HCI_HOST_BUSY_PAIRING,
    HciRejNoSuitableChannel = esp_bt_status_t_ESP_BT_STATUS_HCI_REJ_NO_SUITABLE_CHANNEL,
    HciControllerBusy = esp_bt_status_t_ESP_BT_STATUS_HCI_CONTROLLER_BUSY,
    HciUnsupportedConnectionInterval = esp_bt_status_t_ESP_BT_STATUS_HCI_UNACCEPT_CONN_INTERVAL,
    HciDirectedAdvertisingTimeout = esp_bt_status_t_ESP_BT_STATUS_HCI_DIRECTED_ADVERTISING_TIMEOUT,
    HciConnectionTimeoutDueToMiscFailure =
        esp_bt_status_t_ESP_BT_STATUS_HCI_CONN_TOUT_DUE_TO_MIC_FAILURE,
    HciConnectionEstablishmentFailed = esp_bt_status_t_ESP_BT_STATUS_HCI_CONN_FAILED_ESTABLISHMENT,
    HciMacConnectionFailed = esp_bt_status_t_ESP_BT_STATUS_HCI_MAC_CONNECTION_FAILED,
}

static MEM_FREED: mutex::Mutex<bool> = mutex::Mutex::new(false);

pub fn reduce_bt_memory<'d, B: BluetoothModemPeripheral + 'd>(_modem: B) -> Result<(), EspError> {
    let mut mem_freed = MEM_FREED.lock();

    if *mem_freed {
        Err(EspError::from_infallible::<ESP_ERR_INVALID_STATE>())?;
    }

    #[cfg(esp_idf_btdm_ctrl_mode_br_edr_only)]
    esp!(unsafe { esp_bt_mem_release(esp_bt_mode_t_ESP_BT_MODE_BLE) })?;

    #[cfg(esp_idf_btdm_ctrl_mode_br_ble_only)]
    esp!(unsafe { esp_bt_mem_release(esp_bt_mode_t_ESP_BT_MODE_CLASSIC_BT) })?;

    *mem_freed = true;

    Ok(())
}

#[cfg(esp_idf_btdm_ctrl_mode_btdm)]
pub fn free_bt_memory<B: BluetoothModemPeripheral>(_modem: B) -> Result<(), EspError> {
    let mut mem_freed = MEM_FREED.lock();

    if *mem_freed {
        Err(EspError::from_infallible::<ESP_ERR_INVALID_STATE>())?;
    }

    esp!(unsafe { esp_bt_mem_release(esp_bt_mode_t_ESP_BT_MODE_BTDM) })?;

    *mem_freed = true;

    Ok(())
}

pub struct BtDriver<'d, M>
where
    M: BtMode,
{
    #[cfg(all(feature = "alloc", esp_idf_comp_nvs_flash_enabled))]
    _nvs: Option<EspDefaultNvsPartition>,
    _p: PhantomData<&'d mut ()>,
    _m: PhantomData<M>,
}

impl<'d, M> BtDriver<'d, M>
where
    M: BtMode,
{
    #[cfg(all(feature = "alloc", esp_idf_comp_nvs_flash_enabled))]
    pub fn new<B: BluetoothModemPeripheral + 'd>(
        _modem: B,
        nvs: Option<EspDefaultNvsPartition>,
    ) -> Result<Self, EspError> {
        Self::init(nvs.is_some())?;

        Ok(Self {
            _nvs: nvs,
            _p: PhantomData,
            _m: PhantomData,
        })
    }

    #[cfg(not(all(feature = "alloc", esp_idf_comp_nvs_flash_enabled)))]
    pub fn new<B: BluetoothModemPeripheral + 'd>(_modem: B) -> Result<Self, EspError> {
        Self::init(false)?;

        Ok(Self {
            _p: PhantomData,
            _m: PhantomData,
        })
    }

    fn init(_nvs_enabled: bool) -> Result<(), EspError> {
        crate::private::bt_controller::init(M::mode(), _nvs_enabled)?;

        info!("Init bluedroid");
        esp!(unsafe { esp_bluedroid_init() })?;

        info!("Enable bluedroid");
        esp!(unsafe { esp_bluedroid_enable() })?;

        Ok(())
    }

    #[deprecated(
        since = "0.52.0",
        note = "use `EspGap::set_device_name` or `EspBleGap::set_device_name` instead"
    )]
    #[cfg(not(esp_idf_version_at_least_6_0_0))]
    pub fn set_device_name(&self, device_name: &str) -> Result<(), EspError> {
        use crate::private::cstr::to_cstring_arg;

        let device_name = to_cstring_arg(device_name)?;

        esp!(unsafe { esp_bt_dev_set_device_name(device_name.as_ptr()) })
    }
}

impl<M> Drop for BtDriver<'_, M>
where
    M: BtMode,
{
    fn drop(&mut self) {
        let _ = esp!(unsafe { esp_bluedroid_disable() });

        esp!(unsafe { esp_bluedroid_deinit() }).unwrap();

        esp!(unsafe { esp_bt_controller_disable() }).unwrap();

        esp!(unsafe { esp_bt_controller_deinit() }).unwrap();
    }
}

unsafe impl<M> Send for BtDriver<'_, M> where M: BtMode {}
unsafe impl<M> Sync for BtDriver<'_, M> where M: BtMode {}
