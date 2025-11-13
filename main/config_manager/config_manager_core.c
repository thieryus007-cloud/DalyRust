/**
 * @file config_manager_core.c
 * @brief Core initialization and lifecycle management for config_manager
 *
 * Contains:
 * - Global state variable definitions
 * - Initialization and deinitialization
 * - Mutex management
 * - Public getter/setter functions
 * - Helper functions for device name, poll interval, random generation
 */

#include "config_manager.h"
#include "config_manager_private.h"
#include "config_manager_nvs.h"
#include "config_manager_mqtt.h"
#include "config_manager_registers.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <time.h>

#include "esp_log.h"

#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include "app_config.h"
#include "uart_bms.h"
#include "can_config_defaults.h"

#ifdef ESP_PLATFORM
#include "esp_system.h"
#include "esp_spiffs.h"
#endif

// ============================================================================
// Global State Variables
// ============================================================================

static const char *TAG = "config_manager";

// MQTT configuration and topics
static mqtt_client_config_t s_mqtt_config = {
    .broker_uri = CONFIG_MANAGER_MQTT_DEFAULT_URI,
    .username = CONFIG_MANAGER_MQTT_DEFAULT_USERNAME,
    .password = CONFIG_MANAGER_MQTT_DEFAULT_PASSWORD,
    .client_cert_path = CONFIG_MANAGER_MQTT_DEFAULT_CLIENT_CERT,
    .ca_cert_path = CONFIG_MANAGER_MQTT_DEFAULT_CA_CERT,
    .keepalive_seconds = CONFIG_MANAGER_MQTT_DEFAULT_KEEPALIVE,
    .default_qos = CONFIG_MANAGER_MQTT_DEFAULT_QOS,
    .retain_enabled = CONFIG_MANAGER_MQTT_DEFAULT_RETAIN,
    .verify_hostname = CONFIG_MANAGER_MQTT_DEFAULT_VERIFY_HOSTNAME,
};

static config_manager_mqtt_topics_t s_mqtt_topics = {0};
static bool s_mqtt_topics_loaded = false;

// Thread-safe snapshots for returning configuration data
static mqtt_client_config_t s_mqtt_config_snapshot = {0};
static config_manager_mqtt_topics_t s_mqtt_topics_snapshot = {0};
static config_manager_device_settings_t s_device_settings_snapshot = {0};
static config_manager_uart_pins_t s_uart_pins_snapshot = {0};
static config_manager_wifi_settings_t s_wifi_settings_snapshot = {0};
static config_manager_can_settings_t s_can_settings_snapshot = {0};
static char s_device_name_snapshot[CONFIG_MANAGER_DEVICE_NAME_MAX_LENGTH] = {0};

// Device settings
static config_manager_device_settings_t s_device_settings = {
    .name = APP_DEVICE_NAME,
};

// UART configuration
static config_manager_uart_pins_t s_uart_pins = {
    .tx_gpio = CONFIG_TINYBMS_UART_TX_GPIO,
    .rx_gpio = CONFIG_TINYBMS_UART_RX_GPIO,
};

// WiFi settings
static config_manager_wifi_settings_t s_wifi_settings = {
    .sta = {
        .ssid = CONFIG_TINYBMS_WIFI_STA_SSID,
        .password = CONFIG_TINYBMS_WIFI_STA_PASSWORD,
        .hostname = CONFIG_TINYBMS_WIFI_STA_HOSTNAME,
        .max_retry = CONFIG_TINYBMS_WIFI_STA_MAX_RETRY,
    },
    .ap = {
        .ssid = CONFIG_TINYBMS_WIFI_AP_SSID,
        .password = CONFIG_TINYBMS_WIFI_AP_PASSWORD,
        .channel = CONFIG_TINYBMS_WIFI_AP_CHANNEL,
        .max_clients = CONFIG_TINYBMS_WIFI_AP_MAX_CLIENTS,
    },
};

static char s_wifi_ap_secret[CONFIG_MANAGER_WIFI_PASSWORD_MAX_LENGTH] = {0};
static bool s_wifi_ap_secret_loaded = false;

// CAN settings
static config_manager_can_settings_t s_can_settings = {
    .twai = {
        .tx_gpio = CONFIG_TINYBMS_CAN_VICTRON_TX_GPIO,
        .rx_gpio = CONFIG_TINYBMS_CAN_VICTRON_RX_GPIO,
    },
    .keepalive = {
        .interval_ms = CONFIG_TINYBMS_CAN_KEEPALIVE_INTERVAL_MS,
        .timeout_ms = CONFIG_TINYBMS_CAN_KEEPALIVE_TIMEOUT_MS,
        .retry_ms = CONFIG_TINYBMS_CAN_KEEPALIVE_RETRY_MS,
    },
    .publisher = {
        .period_ms = CONFIG_TINYBMS_CAN_PUBLISHER_PERIOD_MS,
    },
    .identity = {
        .handshake_ascii = CONFIG_TINYBMS_CAN_HANDSHAKE_ASCII,
        .manufacturer = CONFIG_TINYBMS_CAN_MANUFACTURER,
        .battery_name = CONFIG_TINYBMS_CAN_BATTERY_NAME,
        .battery_family = CONFIG_TINYBMS_CAN_BATTERY_FAMILY,
        .serial_number = CONFIG_TINYBMS_CAN_SERIAL_NUMBER,
    },
};

// Configuration state flags
static bool s_config_file_loaded = false;
#ifdef ESP_PLATFORM
static bool s_spiffs_mounted = false;
#endif

// Event publisher callback
static event_bus_publish_fn_t s_event_publisher = NULL;

// Configuration snapshots (used for JSON serialization)
static char s_config_json_full[CONFIG_MANAGER_MAX_CONFIG_SIZE] = {0};
static size_t s_config_length_full = 0;
static char s_config_json_public[CONFIG_MANAGER_MAX_CONFIG_SIZE] = {0};
static size_t s_config_length_public = 0;

// Register management
static uint16_t s_register_raw_values[s_register_count];
static bool s_registers_initialised = false;
static char s_register_events[CONFIG_MANAGER_REGISTER_EVENT_BUFFERS][CONFIG_MANAGER_MAX_UPDATE_PAYLOAD];
static size_t s_next_register_event = 0;

// UART poll interval
static uint32_t s_uart_poll_interval_ms = UART_BMS_DEFAULT_POLL_INTERVAL_MS;

// Persistent settings state
static bool s_settings_loaded = false;

#ifdef ESP_PLATFORM
static bool s_nvs_initialised = false;
#endif

// Mutex to protect access to global configuration state
// NOTE: Currently protects write operations (setters) only.
// TODO: Full thread safety requires protecting all config structure access
static SemaphoreHandle_t s_config_mutex = NULL;
static const TickType_t CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS = pdMS_TO_TICKS(1000);

// ============================================================================
// Mutex Management Functions
// ============================================================================

static esp_err_t config_manager_lock(TickType_t timeout)
{
    if (s_config_mutex == NULL) {
        ESP_LOGE(TAG, "Config mutex not initialized");
        return ESP_ERR_INVALID_STATE;
    }

    if (xSemaphoreTake(s_config_mutex, timeout) != pdTRUE) {
        ESP_LOGW(TAG, "Failed to acquire config mutex");
        return ESP_ERR_TIMEOUT;
    }

    return ESP_OK;
}

static void config_manager_unlock(void)
{
    if (s_config_mutex != NULL) {
        xSemaphoreGive(s_config_mutex);
    }
}

// ============================================================================
// Helper Functions
// ============================================================================

static const char *config_manager_effective_device_name(void)
{
    if (s_device_settings.name[0] != '\0') {
        return s_device_settings.name;
    }
    return APP_DEVICE_NAME;
}

uint32_t config_manager_clamp_poll_interval(uint32_t interval_ms)
{
    if (interval_ms < UART_BMS_MIN_POLL_INTERVAL_MS) {
        return UART_BMS_MIN_POLL_INTERVAL_MS;
    }
    if (interval_ms > UART_BMS_MAX_POLL_INTERVAL_MS) {
        return UART_BMS_MAX_POLL_INTERVAL_MS;
    }
    return interval_ms;
}

void config_manager_generate_random_bytes(uint8_t *buffer, size_t length)
{
    if (buffer == NULL || length == 0) {
        return;
    }

#ifdef ESP_PLATFORM
    esp_fill_random(buffer, length);
#else
    static bool seeded = false;
    if (!seeded) {
        seeded = true;
        unsigned int seed = (unsigned int)time(NULL);
        seed ^= (unsigned int)clock();
        srand(seed);
    }
    for (size_t i = 0; i < length; ++i) {
        buffer[i] = (uint8_t)(rand() & 0xFF);
    }
#endif
}

void config_manager_generate_ap_secret(char *out, size_t out_size)
{
    if (out == NULL || out_size == 0) {
        return;
    }

    static const char alphabet[] = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
    const size_t alphabet_len = sizeof(alphabet) - 1U;
    if (alphabet_len == 0U) {
        out[0] = '\0';
        return;
    }

    uint8_t random_bytes[CONFIG_MANAGER_WIFI_AP_SECRET_LENGTH];
    memset(random_bytes, 0, sizeof(random_bytes));
    config_manager_generate_random_bytes(random_bytes, sizeof(random_bytes));

    size_t required = CONFIG_MANAGER_WIFI_AP_SECRET_LENGTH + 1U;
    if (out_size < required) {
        required = out_size;
    }

    size_t limit = required - 1U;
    for (size_t i = 0; i < limit; ++i) {
        out[i] = alphabet[random_bytes[i] % alphabet_len];
    }
    out[limit] = '\0';
}

void config_manager_apply_ap_secret_if_needed(config_manager_wifi_settings_t *wifi)
{
    if (wifi == NULL) {
        return;
    }

    size_t password_len = strnlen(wifi->ap.password, sizeof(wifi->ap.password));
    if (password_len >= CONFIG_MANAGER_WIFI_PASSWORD_MIN_LENGTH) {
        return;
    }

    config_manager_ensure_ap_secret_loaded();
    if (strlen(s_wifi_ap_secret) >= CONFIG_MANAGER_WIFI_PASSWORD_MIN_LENGTH) {
        config_manager_copy_string(wifi->ap.password,
                                   sizeof(wifi->ap.password),
                                   s_wifi_ap_secret);
    } else {
        ESP_LOGW(TAG, "No valid AP secret available; fallback AP will remain disabled");
    }
}

// Forward declarations of functions needed by config_manager_ensure_initialised
// These are implemented in other modules (config_manager_registers.c, config_manager_nvs.c, etc.)
extern void config_manager_load_register_defaults(void);
extern esp_err_t config_manager_build_config_snapshot(void);

static void config_manager_ensure_initialised(void)
{
    // Initialize mutex on first call (thread-safe in FreeRTOS)
    if (s_config_mutex == NULL) {
        s_config_mutex = xSemaphoreCreateMutex();
        if (s_config_mutex == NULL) {
            ESP_LOGE(TAG, "Failed to create config mutex");
        }
    }

    if (!s_registers_initialised) {
        config_manager_load_register_defaults();
    }

    if (!s_settings_loaded) {
        config_manager_load_persistent_settings();
    }

    if (s_config_length_public == 0) {
        if (config_manager_build_config_snapshot() != ESP_OK) {
            ESP_LOGW(TAG, "Failed to build default configuration snapshot");
        }
    }
}

// ============================================================================
// Public API: Initialization and Lifecycle
// ============================================================================

void config_manager_set_event_publisher(event_bus_publish_fn_t publisher)
{
    s_event_publisher = publisher;
}

void config_manager_init(void)
{
    config_manager_ensure_initialised();
    uart_bms_set_poll_interval_ms(s_uart_poll_interval_ms);
}

void config_manager_deinit(void)
{
    ESP_LOGI(TAG, "Deinitializing config manager...");

#ifdef ESP_PLATFORM
    // Unmount SPIFFS if mounted
    if (s_spiffs_mounted) {
        esp_err_t err = esp_vfs_spiffs_unregister(NULL);
        if (err != ESP_OK) {
            ESP_LOGW(TAG, "Failed to unmount SPIFFS: %s", esp_err_to_name(err));
        } else {
            ESP_LOGI(TAG, "SPIFFS unmounted");
        }
        s_spiffs_mounted = false;
    }
#endif

    // Destroy mutex
    if (s_config_mutex != NULL) {
        vSemaphoreDelete(s_config_mutex);
        s_config_mutex = NULL;
    }

    // Reset state
    s_event_publisher = NULL;
    s_config_length_full = 0;
    s_config_length_public = 0;
    s_registers_initialised = false;
    s_settings_loaded = false;
#ifdef ESP_PLATFORM
    s_nvs_initialised = false;
#endif
    s_mqtt_topics_loaded = false;
    s_config_file_loaded = false;
    s_next_register_event = 0;
    s_uart_poll_interval_ms = UART_BMS_DEFAULT_POLL_INTERVAL_MS;
    memset(s_config_json_full, 0, sizeof(s_config_json_full));
    memset(s_config_json_public, 0, sizeof(s_config_json_public));
    memset(s_register_raw_values, 0, sizeof(s_register_raw_values));
    memset(s_register_events, 0, sizeof(s_register_events));
    memset(&s_mqtt_config, 0, sizeof(s_mqtt_config));
    memset(&s_mqtt_topics, 0, sizeof(s_mqtt_topics));
    memset(&s_device_settings, 0, sizeof(s_device_settings));
    memset(&s_uart_pins, 0, sizeof(s_uart_pins));
    memset(&s_wifi_settings, 0, sizeof(s_wifi_settings));
    memset(&s_can_settings, 0, sizeof(s_can_settings));
    memset(s_wifi_ap_secret, 0, sizeof(s_wifi_ap_secret));
    s_wifi_ap_secret_loaded = false;

    ESP_LOGI(TAG, "Config manager deinitialized");
}

// ============================================================================
// Public API: Getter Functions
// ============================================================================

const config_manager_device_settings_t *config_manager_get_device_settings(void)
{
    config_manager_ensure_initialised();
    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning device settings without lock");
        return &s_device_settings;
    }

    s_device_settings_snapshot = s_device_settings;
    config_manager_unlock();
    return &s_device_settings_snapshot;
}

const char *config_manager_get_device_name(void)
{
    config_manager_ensure_initialised();
    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning device name without lock");
        return config_manager_effective_device_name();
    }

    const char *effective = config_manager_effective_device_name();
    config_manager_copy_string(s_device_name_snapshot,
                               sizeof(s_device_name_snapshot),
                               effective);
    config_manager_unlock();
    return s_device_name_snapshot;
}

uint32_t config_manager_get_uart_poll_interval_ms(void)
{
    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning default UART interval due to lock failure");
        return UART_BMS_DEFAULT_POLL_INTERVAL_MS;
    }

    uint32_t interval = s_uart_poll_interval_ms;
    config_manager_unlock();
    return interval;
}

const config_manager_uart_pins_t *config_manager_get_uart_pins(void)
{
    config_manager_ensure_initialised();
    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning UART pins without lock");
        return &s_uart_pins;
    }

    s_uart_pins_snapshot = s_uart_pins;
    config_manager_unlock();
    return &s_uart_pins_snapshot;
}

const config_manager_wifi_settings_t *config_manager_get_wifi_settings(void)
{
    config_manager_ensure_initialised();
    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning WiFi settings without lock");
        return &s_wifi_settings;
    }

    s_wifi_settings_snapshot = s_wifi_settings;
    config_manager_unlock();
    return &s_wifi_settings_snapshot;
}

const config_manager_can_settings_t *config_manager_get_can_settings(void)
{
    config_manager_ensure_initialised();
    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning CAN settings without lock");
        return &s_can_settings;
    }

    s_can_settings_snapshot = s_can_settings;
    config_manager_unlock();
    return &s_can_settings_snapshot;
}

const char *config_manager_mask_secret(const char *value)
{
    if (value == NULL || value[0] == '\0') {
        return "";
    }
    return CONFIG_MANAGER_SECRET_MASK;
}

// ============================================================================
// Public API: Setter Functions
// ============================================================================

// Forward declarations of functions needed by setter implementations
extern esp_err_t config_manager_store_poll_interval(uint32_t interval_ms);
extern esp_err_t config_manager_build_config_snapshot_locked(void);
extern void config_manager_publish_config_snapshot(void);
extern esp_err_t config_manager_save_config_file(void);

esp_err_t config_manager_set_uart_poll_interval_ms(uint32_t interval_ms)
{
    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    uint32_t clamped = config_manager_clamp_poll_interval(interval_ms);
    if (clamped == s_uart_poll_interval_ms) {
        config_manager_unlock();
        uart_bms_set_poll_interval_ms(clamped);
        return ESP_OK;
    }

    s_uart_poll_interval_ms = clamped;
    uart_bms_set_poll_interval_ms(clamped);

    esp_err_t persist_err = config_manager_store_poll_interval(clamped);
    if (persist_err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to persist UART poll interval: %s", esp_err_to_name(persist_err));
    }

    esp_err_t snapshot_err = config_manager_build_config_snapshot_locked();
    if (snapshot_err == ESP_OK) {
        config_manager_publish_config_snapshot();
        if (persist_err == ESP_OK && s_config_file_loaded) {
            esp_err_t save_err = config_manager_save_config_file();
            if (save_err != ESP_OK) {
                ESP_LOGW(TAG, "Failed to update configuration file: %s", esp_err_to_name(save_err));
            }
        }
    }

    config_manager_unlock();

    if (persist_err != ESP_OK) {
        return persist_err;
    }
    return snapshot_err;
}
