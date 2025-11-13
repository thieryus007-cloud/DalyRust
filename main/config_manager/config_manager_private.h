#pragma once

/**
 * @file config_manager_private.h
 * @brief Private/shared declarations for config_manager module
 *
 * Contains constants, macros, and internal declarations shared between
 * config_manager.c and config_manager_nvs.c
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"
#include "mqtt_client.h"
#include "config_manager.h"

#ifdef ESP_PLATFORM
#include "nvs.h"
#endif

// ============================================================================
// Configuration Constants (lines 36-148 from config_manager.c)
// ============================================================================

#define CONFIG_MANAGER_REGISTER_EVENT_BUFFERS 4
#define CONFIG_MANAGER_MAX_UPDATE_PAYLOAD     192
#define CONFIG_MANAGER_MAX_REGISTER_KEY       32
#define CONFIG_MANAGER_NAMESPACE              "gateway_cfg"
#define CONFIG_MANAGER_POLL_KEY               "uart_poll"
#define CONFIG_MANAGER_REGISTER_KEY_PREFIX    "reg"
#define CONFIG_MANAGER_REGISTER_KEY_MAX       16

#define CONFIG_MANAGER_MQTT_URI_KEY          "mqtt_uri"
#define CONFIG_MANAGER_MQTT_USERNAME_KEY     "mqtt_user"
#define CONFIG_MANAGER_MQTT_PASSWORD_KEY     "mqtt_pass"
#define CONFIG_MANAGER_MQTT_KEEPALIVE_KEY    "mqtt_keepalive"
#define CONFIG_MANAGER_MQTT_QOS_KEY          "mqtt_qos"
#define CONFIG_MANAGER_MQTT_RETAIN_KEY       "mqtt_retain"
#define CONFIG_MANAGER_MQTT_TLS_CLIENT_KEY   "mqtt_tls_cli"
#define CONFIG_MANAGER_MQTT_TLS_CA_KEY       "mqtt_tls_ca"
#define CONFIG_MANAGER_MQTT_TLS_VERIFY_KEY   "mqtt_tls_vrf"
#define CONFIG_MANAGER_MQTT_TOPIC_STATUS_KEY "mqtt_t_stat"
#define CONFIG_MANAGER_MQTT_TOPIC_MET_KEY    "mqtt_t_met"
#define CONFIG_MANAGER_MQTT_TOPIC_CFG_KEY    "mqtt_t_cfg"
#define CONFIG_MANAGER_MQTT_TOPIC_RAW_KEY    "mqtt_t_crw"
#define CONFIG_MANAGER_MQTT_TOPIC_DEC_KEY    "mqtt_t_cdc"
#define CONFIG_MANAGER_MQTT_TOPIC_RDY_KEY    "mqtt_t_crd"
#define CONFIG_MANAGER_WIFI_AP_SECRET_KEY    "wifi_ap_secret"

#ifndef CONFIG_TINYBMS_MQTT_BROKER_URI
#define CONFIG_TINYBMS_MQTT_BROKER_URI "mqtt://localhost"
#endif

#ifndef CONFIG_TINYBMS_MQTT_USERNAME
#define CONFIG_TINYBMS_MQTT_USERNAME ""
#endif

#ifndef CONFIG_TINYBMS_MQTT_PASSWORD
#define CONFIG_TINYBMS_MQTT_PASSWORD ""
#endif

#ifndef CONFIG_TINYBMS_MQTT_KEEPALIVE
#define CONFIG_TINYBMS_MQTT_KEEPALIVE 60
#endif

#ifndef CONFIG_TINYBMS_MQTT_DEFAULT_QOS
#define CONFIG_TINYBMS_MQTT_DEFAULT_QOS 1
#endif

#ifndef CONFIG_TINYBMS_MQTT_RETAIN_STATUS
#define CONFIG_TINYBMS_MQTT_RETAIN_STATUS 0
#endif

#define CONFIG_MANAGER_MQTT_DEFAULT_URI       CONFIG_TINYBMS_MQTT_BROKER_URI
#define CONFIG_MANAGER_MQTT_DEFAULT_USERNAME  CONFIG_TINYBMS_MQTT_USERNAME
#define CONFIG_MANAGER_MQTT_DEFAULT_PASSWORD  CONFIG_TINYBMS_MQTT_PASSWORD
#define CONFIG_MANAGER_MQTT_DEFAULT_KEEPALIVE ((uint16_t)CONFIG_TINYBMS_MQTT_KEEPALIVE)
#define CONFIG_MANAGER_MQTT_DEFAULT_QOS       ((uint8_t)CONFIG_TINYBMS_MQTT_DEFAULT_QOS)
#define CONFIG_MANAGER_MQTT_DEFAULT_RETAIN          (CONFIG_TINYBMS_MQTT_RETAIN_STATUS != 0)
#define CONFIG_MANAGER_MQTT_DEFAULT_CLIENT_CERT     ""
#define CONFIG_MANAGER_MQTT_DEFAULT_CA_CERT         ""
#define CONFIG_MANAGER_MQTT_DEFAULT_VERIFY_HOSTNAME true

#define CONFIG_MANAGER_FS_BASE_PATH "/spiffs"
#define CONFIG_MANAGER_CONFIG_FILE  CONFIG_MANAGER_FS_BASE_PATH "/config.json"

#ifndef CONFIG_TINYBMS_WIFI_STA_SSID
#define CONFIG_TINYBMS_WIFI_STA_SSID ""
#endif

#ifndef CONFIG_TINYBMS_WIFI_STA_PASSWORD
#define CONFIG_TINYBMS_WIFI_STA_PASSWORD ""
#endif

#ifndef CONFIG_TINYBMS_WIFI_STA_HOSTNAME
#define CONFIG_TINYBMS_WIFI_STA_HOSTNAME ""
#endif

#ifndef CONFIG_TINYBMS_WIFI_STA_MAX_RETRY
#define CONFIG_TINYBMS_WIFI_STA_MAX_RETRY 5
#endif

#ifndef CONFIG_TINYBMS_WIFI_AP_SSID
#define CONFIG_TINYBMS_WIFI_AP_SSID "TinyBMS-Gateway"
#endif

#ifndef CONFIG_TINYBMS_WIFI_AP_PASSWORD
#define CONFIG_TINYBMS_WIFI_AP_PASSWORD ""
#endif

#define CONFIG_MANAGER_WIFI_PASSWORD_MIN_LENGTH 8U
#define CONFIG_MANAGER_WIFI_AP_SECRET_LENGTH    16U

#ifndef CONFIG_TINYBMS_WIFI_ENABLE
#define CONFIG_TINYBMS_WIFI_ENABLE 1
#endif

#ifndef CONFIG_TINYBMS_WIFI_AP_CHANNEL
#define CONFIG_TINYBMS_WIFI_AP_CHANNEL 1
#endif

#ifndef CONFIG_TINYBMS_WIFI_AP_MAX_CLIENTS
#define CONFIG_TINYBMS_WIFI_AP_MAX_CLIENTS 4
#endif

#ifndef CONFIG_TINYBMS_UART_TX_GPIO
#define CONFIG_TINYBMS_UART_TX_GPIO 37
#endif

#ifndef CONFIG_TINYBMS_UART_RX_GPIO
#define CONFIG_TINYBMS_UART_RX_GPIO 36
#endif

#ifndef CONFIG_TINYBMS_CAN_SERIAL_NUMBER
#define CONFIG_TINYBMS_CAN_SERIAL_NUMBER "TinyBMS-00000000"
#endif

// ============================================================================
// Extern Declarations for Global State Variables
// ============================================================================

extern const char *TAG;

// WiFi AP secret for NVS persistence
extern char s_wifi_ap_secret[CONFIG_MANAGER_WIFI_PASSWORD_MAX_LENGTH];
extern bool s_wifi_ap_secret_loaded;

// MQTT configuration and topics
extern mqtt_client_config_t s_mqtt_config;
extern config_manager_mqtt_topics_t s_mqtt_topics;

// NVS initialization flag
#ifdef ESP_PLATFORM
extern bool s_nvs_initialised;
#endif

// SPIFFS mount flag
#ifdef ESP_PLATFORM
extern bool s_spiffs_mounted;
#endif

// Configuration file loaded flag
extern bool s_config_file_loaded;

// Settings loaded flag
extern bool s_settings_loaded;

// UART poll interval
extern uint32_t s_uart_poll_interval_ms;

// WiFi settings
extern config_manager_wifi_settings_t s_wifi_settings;

// Register-related types and variables
typedef enum {
    CONFIG_MANAGER_ACCESS_RO = 0,
    CONFIG_MANAGER_ACCESS_WO,
    CONFIG_MANAGER_ACCESS_RW,
} config_manager_access_t;

typedef enum {
    CONFIG_MANAGER_VALUE_NUMERIC = 0,
    CONFIG_MANAGER_VALUE_ENUM,
} config_manager_value_class_t;

typedef struct {
    uint16_t value;
    const char *label;
} config_manager_enum_entry_t;

typedef struct {
    uint16_t address;
    const char *key;
    const char *label;
    const char *unit;
    const char *group;
    const char *comment;
    const char *type;
    config_manager_access_t access;
    float scale;
    uint8_t precision;
    bool has_min;
    uint16_t min_raw;
    bool has_max;
    uint16_t max_raw;
    float step_raw;
    uint16_t default_raw;
    config_manager_value_class_t value_class;
    const config_manager_enum_entry_t *enum_values;
    size_t enum_count;
} config_manager_register_descriptor_t;

// Note: s_register_descriptors and s_register_raw_values are defined via
// #include "generated_tiny_rw_registers.inc" in config_manager.c
extern const config_manager_register_descriptor_t s_register_descriptors[];
extern const size_t s_register_count;
extern uint16_t *s_register_raw_values;

// ============================================================================
// Shared Utility Function Declarations
// ============================================================================

/**
 * @brief Copy string with bounds checking
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string (can be NULL)
 */
void config_manager_copy_string(char *dest, size_t dest_size, const char *src);

/**
 * @brief Load MQTT settings from NVS handle
 * @param handle NVS handle (ESP_PLATFORM only)
 */
#ifdef ESP_PLATFORM
void config_manager_load_mqtt_settings_from_nvs(nvs_handle_t handle);
#else
void config_manager_load_mqtt_settings_from_nvs(void);
#endif

/**
 * @brief Ensure MQTT topics are loaded with defaults
 */
void config_manager_ensure_topics_loaded(void);

/**
 * @brief Sanitize MQTT configuration values
 * @param config MQTT config structure to sanitize
 */
void config_manager_sanitise_mqtt_config(mqtt_client_config_t *config);

/**
 * @brief Sanitize MQTT topics values
 * @param topics MQTT topics structure to sanitize
 */
void config_manager_sanitise_mqtt_topics(config_manager_mqtt_topics_t *topics);

/**
 * @brief Clamp poll interval to valid range
 * @param interval_ms Interval in milliseconds
 * @return Clamped interval value
 */
uint32_t config_manager_clamp_poll_interval(uint32_t interval_ms);

/**
 * @brief Generate random bytes for secret generation
 * @param buffer Output buffer
 * @param length Number of bytes to generate
 */
void config_manager_generate_random_bytes(uint8_t *buffer, size_t length);

/**
 * @brief Generate WiFi AP secret string
 * @param out Output buffer
 * @param out_size Size of output buffer
 */
void config_manager_generate_ap_secret(char *out, size_t out_size);

/**
 * @brief Apply AP secret to WiFi settings if needed
 * @param wifi WiFi settings structure
 */
void config_manager_apply_ap_secret_if_needed(config_manager_wifi_settings_t *wifi);

/**
 * @brief Load configuration file from SPIFFS
 * @param apply_runtime Whether to apply runtime settings
 * @return ESP_OK on success, ESP_ERR_NOT_FOUND if file doesn't exist
 */
esp_err_t config_manager_load_config_file(bool apply_runtime);

/**
 * @brief Align raw register value according to descriptor constraints
 * @param desc Register descriptor
 * @param user_value User-provided value
 * @param out_raw Pointer to store aligned raw value
 * @return ESP_OK on success
 */
esp_err_t config_manager_align_raw_value(const config_manager_register_descriptor_t *desc,
                                          float user_value,
                                          uint16_t *out_raw);
