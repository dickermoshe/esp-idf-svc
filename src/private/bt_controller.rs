use ::log::info;

use crate::sys::*;

#[allow(clippy::needless_update)]
pub fn init(mode: esp_bt_mode_t, _nvs_enabled: bool) -> Result<(), EspError> {
    #[cfg(esp32)]
    let mut bt_cfg = esp_bt_controller_config_t {
        magic: crate::sys::ESP_BT_CONTROLLER_CONFIG_MAGIC_VAL,
        controller_task_stack_size: crate::sys::ESP_TASK_BT_CONTROLLER_STACK as _,
        controller_task_prio: crate::sys::ESP_TASK_BT_CONTROLLER_PRIO as _,
        hci_uart_no: crate::sys::BT_HCI_UART_NO_DEFAULT as _,
        hci_uart_baudrate: crate::sys::BT_HCI_UART_BAUDRATE_DEFAULT,
        scan_duplicate_mode: crate::sys::SCAN_DUPLICATE_MODE as _,
        scan_duplicate_type: crate::sys::SCAN_DUPLICATE_TYPE_VALUE as _,
        normal_adv_size: crate::sys::NORMAL_SCAN_DUPLICATE_CACHE_SIZE as _,
        mesh_adv_size: crate::sys::MESH_DUPLICATE_SCAN_CACHE_SIZE as _,
        send_adv_reserved_size: crate::sys::SCAN_SEND_ADV_RESERVED_SIZE as _,
        controller_debug_flag: crate::sys::CONTROLLER_ADV_LOST_DEBUG_BIT,
        mode: mode as _,
        ble_max_conn: crate::sys::CONFIG_BTDM_CTRL_BLE_MAX_CONN_EFF as _,
        bt_max_acl_conn: crate::sys::CONFIG_BTDM_CTRL_BR_EDR_MAX_ACL_CONN_EFF as _,
        bt_sco_datapath: crate::sys::CONFIG_BTDM_CTRL_BR_EDR_SCO_DATA_PATH_EFF as _,
        auto_latency: crate::sys::BTDM_CTRL_AUTO_LATENCY_EFF != 0,
        bt_legacy_auth_vs_evt: crate::sys::BTDM_CTRL_LEGACY_AUTH_VENDOR_EVT_EFF != 0,
        // See https://github.com/espressif/esp-idf/blob/12f36a021f511cd4de41d3fffff146c5336ac1e7/components/bt/controller/esp32/esp_bredr_cfg.h#L19
        // bt_max_sync_conn: crate::sys::CONFIG_BTDM_CTRL_BR_EDR_MAX_SYNC_CONN_EFF as _,
        bt_max_sync_conn: 1,
        ble_sca: crate::sys::CONFIG_BTDM_BLE_SLEEP_CLOCK_ACCURACY_INDEX_EFF as _,
        pcm_role: crate::sys::CONFIG_BTDM_CTRL_PCM_ROLE_EFF as _,
        pcm_polar: crate::sys::CONFIG_BTDM_CTRL_PCM_POLAR_EFF as _,
        hli: crate::sys::BTDM_CTRL_HLI != 0,
        #[cfg(any(
            esp_idf_version_patch_at_least_5_1_7,
            esp_idf_version_patch_at_least_5_2_6,
            esp_idf_version_patch_at_least_5_3_3,
            esp_idf_version_patch_at_least_5_4_1,
            esp_idf_version_at_least_5_5_0,
        ))]
        enc_key_sz_min: crate::sys::CONFIG_BTDM_CTRL_BR_EDR_MIN_ENC_KEY_SZ_DFT_EFF as _,
        dup_list_refresh_period: crate::sys::SCAN_DUPL_CACHE_REFRESH_PERIOD as _,
        ble_scan_backoff: crate::sys::BTDM_CTRL_SCAN_BACKOFF_UPPERLIMITMAX != 0,
        ble_llcp_disc_flag: crate::sys::BTDM_BLE_LLCP_DISC_FLAG as _,
        ble_aa_check: crate::sys::BTDM_CTRL_CHECK_CONNECT_IND_ACCESS_ADDRESS_ENABLED != 0,
        ble_chan_ass_en: crate::sys::BTDM_BLE_CHAN_ASS_EN as _,
        ble_ping_en: crate::sys::BTDM_BLE_PING_EN as _,
        ..Default::default()
    };

    #[cfg(any(esp32c3, esp32s3))]
    let mut bt_cfg = esp_bt_controller_config_t {
        magic: crate::sys::ESP_BT_CTRL_CONFIG_MAGIC_VAL,
        version: crate::sys::ESP_BT_CTRL_CONFIG_VERSION,
        controller_task_stack_size: crate::sys::ESP_TASK_BT_CONTROLLER_STACK as _,
        controller_task_prio: crate::sys::ESP_TASK_BT_CONTROLLER_PRIO as _,
        controller_task_run_cpu: crate::sys::CONFIG_BT_CTRL_PINNED_TO_CORE as _,
        bluetooth_mode: crate::sys::CONFIG_BT_CTRL_MODE_EFF as _,
        ble_max_act: crate::sys::CONFIG_BT_CTRL_BLE_MAX_ACT_EFF as _,
        sleep_mode: crate::sys::CONFIG_BT_CTRL_SLEEP_MODE_EFF as _,
        sleep_clock: crate::sys::CONFIG_BT_CTRL_SLEEP_CLOCK_EFF as _,
        ble_st_acl_tx_buf_nb: crate::sys::CONFIG_BT_CTRL_BLE_STATIC_ACL_TX_BUF_NB as _,
        ble_hw_cca_check: crate::sys::CONFIG_BT_CTRL_HW_CCA_EFF as _,
        ble_adv_dup_filt_max: crate::sys::CONFIG_BT_CTRL_ADV_DUP_FILT_MAX as _,
        ce_len_type: crate::sys::CONFIG_BT_CTRL_CE_LENGTH_TYPE_EFF as _,
        hci_tl_type: crate::sys::CONFIG_BT_CTRL_HCI_TL_EFF as _,
        hci_tl_funcs: core::ptr::null_mut(),
        txant_dft: crate::sys::CONFIG_BT_CTRL_TX_ANTENNA_INDEX_EFF as _,
        rxant_dft: crate::sys::CONFIG_BT_CTRL_RX_ANTENNA_INDEX_EFF as _,
        txpwr_dft: crate::sys::CONFIG_BT_CTRL_DFT_TX_POWER_LEVEL_EFF as _,
        cfg_mask: crate::sys::CFG_MASK,
        scan_duplicate_mode: crate::sys::SCAN_DUPLICATE_MODE as _,
        scan_duplicate_type: crate::sys::SCAN_DUPLICATE_TYPE_VALUE as _,
        normal_adv_size: crate::sys::NORMAL_SCAN_DUPLICATE_CACHE_SIZE as _,
        mesh_adv_size: crate::sys::MESH_DUPLICATE_SCAN_CACHE_SIZE as _,
        coex_phy_coded_tx_rx_time_limit: crate::sys::CONFIG_BT_CTRL_COEX_PHY_CODED_TX_RX_TLIM_EFF
            as _,
        hw_target_code: crate::sys::BLE_HW_TARGET_CODE_CHIP_ECO0 as _,
        slave_ce_len_min: crate::sys::SLAVE_CE_LEN_MIN_DEFAULT as _,
        hw_recorrect_en: crate::sys::AGC_RECORRECT_EN as _,
        cca_thresh: crate::sys::CONFIG_BT_CTRL_HW_CCA_VAL as _,
        coex_param_en: false,
        coex_use_hooks: false,
        scan_backoff_upperlimitmax: crate::sys::BT_CTRL_SCAN_BACKOFF_UPPERLIMITMAX as _,
        dup_list_refresh_period: crate::sys::DUPL_SCAN_CACHE_REFRESH_PERIOD as _,
        ble_50_feat_supp: crate::sys::BT_CTRL_50_FEATURE_SUPPORT != 0,
        ble_cca_mode: crate::sys::BT_BLE_CCA_MODE as _,
        ble_data_lenth_zero_aux: crate::sys::BT_BLE_ADV_DATA_LENGTH_ZERO_AUX as _,
        ble_chan_ass_en: crate::sys::BT_CTRL_CHAN_ASS_EN as _,
        ble_ping_en: crate::sys::BT_CTRL_LE_PING_EN as _,
        ble_llcp_disc_flag: crate::sys::BT_CTRL_BLE_LLCP_DISC_FLAG as _,
        run_in_flash: crate::sys::BT_CTRL_RUN_IN_FLASH_ONLY != 0,
        dtm_en: crate::sys::BT_CTRL_DTM_ENABLE != 0,
        enc_en: crate::sys::BLE_SECURITY_ENABLE != 0,
        qa_test: crate::sys::BT_CTRL_BLE_TEST != 0,
        #[cfg(any(
            esp_idf_version_patch_at_least_5_1_7,
            esp_idf_version_patch_at_least_5_2_6,
            esp_idf_version_patch_at_least_5_3_4,
            esp_idf_version_patch_at_least_5_4_2,
            esp_idf_version_at_least_5_5_0,
        ))]
        connect_en: crate::sys::BT_CTRL_BLE_MASTER != 0,
        scan_en: crate::sys::BT_CTRL_BLE_SCAN != 0,
        ble_aa_check: crate::sys::BLE_CTRL_CHECK_CONNECT_IND_ACCESS_ADDRESS_ENABLED != 0,
        #[cfg(any(
            all(
                esp_idf_version_patch_at_least_5_2_6,
                esp_idf_version_patch_at_most_5_2_6
            ),
            all(
                esp_idf_version_patch_at_least_5_3_3,
                esp_idf_version_patch_at_most_5_3_4
            ),
            all(
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_patch_at_most_5_4_3
            ),
            all(
                esp_idf_version_patch_at_least_5_5_0,
                esp_idf_version_patch_at_most_5_5_1
            ),
        ))]
        ble_log_mode_en: crate::sys::BLE_LOG_MODE_EN,
        #[cfg(any(
            all(
                esp_idf_version_patch_at_least_5_2_6,
                esp_idf_version_patch_at_most_5_2_6
            ),
            all(
                esp_idf_version_patch_at_least_5_3_3,
                esp_idf_version_patch_at_most_5_3_4
            ),
            all(
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_patch_at_most_5_4_3
            ),
            all(
                esp_idf_version_patch_at_least_5_5_0,
                esp_idf_version_patch_at_most_5_5_1
            ),
        ))]
        ble_log_level: crate::sys::BLE_LOG_LEVEL as _,
        #[cfg(any(
            esp_idf_version_patch_at_least_5_1_7,
            esp_idf_version_patch_at_least_5_2_6,
            esp_idf_version_patch_at_least_5_3_4,
            esp_idf_version_patch_at_least_5_4_2,
            esp_idf_version_at_least_5_5_0,
        ))]
        adv_en: crate::sys::BT_CTRL_BLE_ADV != 0,
        ..Default::default()
    };

    #[cfg(not(any(esp32, esp32s3, esp32c3)))]
    let mut bt_cfg = esp_bt_controller_config_t {
        config_version: CONFIG_VERSION as _,
        ble_ll_resolv_list_size: crate::sys::CONFIG_BT_LE_LL_RESOLV_LIST_SIZE as _,
        ble_hci_evt_hi_buf_count: crate::sys::DEFAULT_BT_LE_HCI_EVT_HI_BUF_COUNT as _,
        ble_hci_evt_lo_buf_count: crate::sys::DEFAULT_BT_LE_HCI_EVT_LO_BUF_COUNT as _,
        ble_ll_sync_list_cnt: crate::sys::DEFAULT_BT_LE_MAX_PERIODIC_ADVERTISER_LIST as _,
        ble_ll_sync_cnt: crate::sys::DEFAULT_BT_LE_MAX_PERIODIC_SYNCS as _,
        ble_ll_rsp_dup_list_count: crate::sys::CONFIG_BT_LE_LL_DUP_SCAN_LIST_COUNT as _,
        ble_ll_adv_dup_list_count: crate::sys::CONFIG_BT_LE_LL_DUP_SCAN_LIST_COUNT as _,
        ble_ll_tx_pwr_dbm: crate::sys::BLE_LL_TX_PWR_DBM_N as _,
        rtc_freq: crate::sys::RTC_FREQ_N as _,
        ble_ll_sca: crate::sys::CONFIG_BT_LE_LL_SCA as _,
        ble_ll_scan_phy_number: crate::sys::BLE_LL_SCAN_PHY_NUMBER_N as _,
        ble_ll_conn_def_auth_pyld_tmo: crate::sys::BLE_LL_CONN_DEF_AUTH_PYLD_TMO_N as _,
        ble_ll_jitter_usecs: crate::sys::BLE_LL_JITTER_USECS_N as _,
        ble_ll_sched_max_adv_pdu_usecs: crate::sys::BLE_LL_SCHED_MAX_ADV_PDU_USECS_N as _,
        ble_ll_sched_direct_adv_max_usecs: crate::sys::BLE_LL_SCHED_DIRECT_ADV_MAX_USECS_N as _,
        ble_ll_sched_adv_max_usecs: crate::sys::BLE_LL_SCHED_ADV_MAX_USECS_N as _,
        ble_scan_rsp_data_max_len: crate::sys::DEFAULT_BT_LE_SCAN_RSP_DATA_MAX_LEN_N as _,
        ble_ll_cfg_num_hci_cmd_pkts: crate::sys::BLE_LL_CFG_NUM_HCI_CMD_PKTS_N as _,
        ble_ll_ctrl_proc_timeout_ms: crate::sys::BLE_LL_CTRL_PROC_TIMEOUT_MS_N,
        nimble_max_connections: crate::sys::DEFAULT_BT_LE_MAX_CONNECTIONS as _,
        ble_whitelist_size: crate::sys::DEFAULT_BT_NIMBLE_WHITELIST_SIZE as _,
        ble_acl_buf_size: crate::sys::DEFAULT_BT_LE_ACL_BUF_SIZE as _,
        ble_acl_buf_count: crate::sys::DEFAULT_BT_LE_ACL_BUF_COUNT as _,
        ble_hci_evt_buf_size: crate::sys::DEFAULT_BT_LE_HCI_EVT_BUF_SIZE as _,
        ble_multi_adv_instances: crate::sys::DEFAULT_BT_LE_MAX_EXT_ADV_INSTANCES as _,
        ble_ext_adv_max_size: crate::sys::DEFAULT_BT_LE_EXT_ADV_MAX_SIZE as _,
        controller_task_stack_size: crate::sys::NIMBLE_LL_STACK_SIZE as _,
        controller_task_prio: crate::sys::ESP_TASK_BT_CONTROLLER_PRIO as _,
        controller_run_cpu: 0,
        enable_qa_test: crate::sys::RUN_QA_TEST as _,
        enable_bqb_test: crate::sys::RUN_BQB_TEST as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        enable_uart_hci: crate::sys::HCI_UART_EN as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_port: crate::sys::DEFAULT_BT_LE_HCI_UART_PORT as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_baud: crate::sys::DEFAULT_BT_LE_HCI_UART_BAUD,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_data_bits: crate::sys::DEFAULT_BT_LE_HCI_UART_DATA_BITS as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_stop_bits: crate::sys::DEFAULT_BT_LE_HCI_UART_STOP_BITS as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_flow_ctrl: crate::sys::DEFAULT_BT_LE_HCI_UART_FLOW_CTRL as _,
        #[cfg(any(
            esp_idf_version_major = "4",
            esp_idf_version = "5.0",
            all(esp_idf_version = "5.1", not(esp_idf_version_patch_at_least_5_1_5)),
            all(esp_idf_version = "5.2", not(esp_idf_version_patch_at_least_5_2_3)),
            all(esp_idf_version = "5.3", not(esp_idf_version_patch_at_least_5_3_1)),
            not(any(esp32c5, esp32c6, esp32c61, esp32h2))
        ))]
        ble_hci_uart_uart_parity: crate::sys::DEFAULT_BT_LE_HCI_UART_PARITY as _,
        enable_tx_cca: crate::sys::DEFAULT_BT_LE_TX_CCA_ENABLED as _,
        cca_rssi_thresh: (256 - crate::sys::DEFAULT_BT_LE_CCA_RSSI_THRESH) as _,
        sleep_en: crate::sys::NIMBLE_SLEEP_ENABLE as _,
        coex_phy_coded_tx_rx_time_limit: crate::sys::DEFAULT_BT_LE_COEX_PHY_CODED_TX_RX_TLIM_EFF
            as _,
        dis_scan_backoff: crate::sys::NIMBLE_DISABLE_SCAN_BACKOFF as _,
        // `esp32c2` defaults this to 0; every other chip in this block defaults to 1
        #[cfg(esp32c2)]
        ble_scan_classify_filter_enable: 0,
        #[cfg(not(esp32c2))]
        ble_scan_classify_filter_enable: 1,
        main_xtal_freq: crate::sys::CONFIG_XTAL_FREQ as _,
        #[cfg(esp32c2)]
        version_num: unsafe { crate::sys::esp_ble_get_chip_rev_version() },
        #[cfg(esp32c6)]
        #[allow(clippy::unnecessary_cast)]
        version_num: unsafe { crate::sys::efuse_hal_chip_revision() as _ },
        #[cfg(not(esp32c2))]
        cpu_freq_mhz: crate::sys::CONFIG_ESP_DEFAULT_CPU_FREQ_MHZ as _,
        ignore_wl_for_direct_adv: 0,
        #[cfg(not(esp32c2))]
        enable_pcl: crate::sys::DEFAULT_BT_LE_POWER_CONTROL_ENABLED as _,
        #[cfg(all(
            not(esp_idf_version_major = "4"),
            not(esp_idf_version = "5.0"),
            not(esp_idf_version = "5.1")
        ))]
        csa2_select: crate::sys::DEFAULT_BT_LE_50_FEATURE_SUPPORT as _,
        // The fields below were added to the modern NimBLE controller config (shared by
        // c2/c5/c6/h2) across IDF 5.3.4 / 5.4.2 / 5.5.0. They are absent from `Default`
        // (which zeroes them), and several have non-zero upstream defaults - notably
        // `vhci_enabled`, without which Bluedroid's HCI host layer fails to start. Field
        // availability differs per chip/version, hence the gating below:
        //  - c6/h2 got Group A/B at 5.3.4; c5 got them one minor later, at 5.4.2.
        //  - c2 (minimal controller) has Group A but none of the Group B fields.
        //
        // Group A - c2, c6, h2 (>= 5.3.4) and c5 (>= 5.4.2)
        #[cfg(all(
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        vhci_enabled: crate::sys::DEFAULT_BT_LE_VHCI_ENABLED as _,
        #[cfg(all(
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ble_aa_check: crate::sys::DEFAULT_BT_LE_CTRL_CHECK_CONNECT_IND_ACCESS_ADDRESS as _,
        #[cfg(all(
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ble_llcp_disc_flag: crate::sys::BT_LE_CTRL_LLCP_DISC_FLAG as _,
        #[cfg(all(
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        scan_backoff_upperlimitmax: crate::sys::BT_CTRL_SCAN_BACKOFF_UPPERLIMITMAX as _,
        // Group B - as Group A, but not present on esp32c2
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ble_chan_ass_en: crate::sys::DEFAULT_BT_LE_CTRL_CHAN_ASS_EN as _,
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ble_data_lenth_zero_aux: crate::sys::DEFAULT_BT_LE_CTRL_ADV_DATA_LENGTH_ZERO_AUX as _,
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ptr_check_enabled: crate::sys::DEFAULT_BT_LE_PTR_CHECK_ENABLED as _,
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        fast_conn_data_tx_en: crate::sys::DEFAULT_BT_LE_CTRL_FAST_CONN_DATA_TX_EN as _,
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        ch39_txpwr: crate::sys::BLE_LL_TX_PWR_DBM_N as _,
        // `adv_rsv_cnt` / `conn_rsv_cnt` mirror the controller's `MIN(...)` macros, which
        // bindgen cannot capture as constants, so they are reproduced from their operands.
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        #[allow(clippy::unnecessary_cast)]
        adv_rsv_cnt: core::cmp::min(
            crate::sys::DEFAULT_BT_LE_MAX_EXT_ADV_INSTANCES as u32,
            crate::sys::CONFIG_BT_LE_EXT_ADV_RESERVED_MEMORY_COUNT as u32,
        ) as _,
        #[cfg(all(
            not(esp32c2),
            any(
                esp_idf_version_patch_at_least_5_3_4,
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
            any(
                not(esp32c5),
                esp_idf_version_patch_at_least_5_4_2,
                esp_idf_version_at_least_5_5_0,
            ),
        ))]
        #[allow(clippy::unnecessary_cast)]
        conn_rsv_cnt: core::cmp::min(
            crate::sys::DEFAULT_BT_LE_MAX_CONNECTIONS as u32,
            crate::sys::CONFIG_BT_LE_CONN_RESERVED_MEMORY_COUNT as u32,
        ) as _,
        // Group C - `priority_level_cfg`, added in 5.5.0 (still present on 6.0+), not on c2
        #[cfg(all(not(esp32c2), esp_idf_version_at_least_5_5_0))]
        priority_level_cfg: crate::sys::BT_LL_CTRL_PRIO_LVL_CFG as _,
        // Group D - added in 5.5.0 and removed again in 6.0, not on c2
        #[cfg(all(
            not(esp32c2),
            esp_idf_version_at_least_5_5_0,
            not(esp_idf_version_at_least_6_0_0),
        ))]
        slv_fst_rx_lat_en: crate::sys::DEFAULT_BT_LE_CTRL_SLV_FAST_RX_CONN_DATA_EN as _,
        #[cfg(all(
            not(esp32c2),
            esp_idf_version_at_least_5_5_0,
            not(esp_idf_version_at_least_6_0_0),
        ))]
        dl_itvl_phy_sync_en: crate::sys::DEFAULT_BT_LE_CTRL_DL_ITVL_PHY_SYNC_EN as _,
        #[cfg(all(
            not(esp32c2),
            esp_idf_version_at_least_5_5_0,
            not(esp_idf_version_at_least_6_0_0),
        ))]
        scan_allow_adi_filter: crate::sys::DEFAULT_BT_SCAN_ALLOW_ENH_ADI_FILTER as _,
        config_magic: CONFIG_MAGIC as _,
        ..Default::default()
    };

    info!("Init bluetooth controller");
    esp!(unsafe { esp_bt_controller_init(&mut bt_cfg) })?;

    info!("Enable bluetooth controller");
    esp!(unsafe { esp_bt_controller_enable(mode) })?;

    Ok(())
}
