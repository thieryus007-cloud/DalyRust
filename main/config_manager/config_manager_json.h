#pragma once

/**
 * @file config_manager_json.h
 * @brief JSON serialization and deserialization for configuration manager
 *
 * This module handles all JSON operations for the configuration manager including:
 * - Configuration snapshot building and serialization
 * - JSON parsing and deserialization
 * - Configuration file I/O (/spiffs/config.json)
 * - Event publishing for configuration updates
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"
#include "cJSON.h"

#include "config_manager.h"
#include "event_bus.h"
#include "mqtt_client.h"

#ifdef __cplusplus
extern "C" {
#endif

// Forward declarations for internal types
typedef struct {
    char name[CONFIG_MANAGER_DEVICE_NAME_MAX_LENGTH];
} config_manager_device_settings_internal_t;

typedef struct {
    int tx_gpio;
    int rx_gpio;
} config_manager_uart_pins_internal_t;

typedef struct {
    struct {
        char ssid[CONFIG_MANAGER_WIFI_SSID_MAX_LENGTH];
        char password[CONFIG_MANAGER_WIFI_PASSWORD_MAX_LENGTH];
        char hostname[CONFIG_MANAGER_WIFI_HOSTNAME_MAX_LENGTH];
        uint8_t max_retry;
    } sta;
    struct {
        char ssid[CONFIG_MANAGER_WIFI_SSID_MAX_LENGTH];
        char password[CONFIG_MANAGER_WIFI_PASSWORD_MAX_LENGTH];
        uint8_t channel;
        uint8_t max_clients;
    } ap;
} config_manager_wifi_settings_internal_t;

typedef struct {
    struct {
        int tx_gpio;
        int rx_gpio;
    } twai;
    struct {
        uint32_t interval_ms;
        uint32_t timeout_ms;
        uint32_t retry_ms;
    } keepalive;
    struct {
        uint32_t period_ms;
    } publisher;
    struct {
        char handshake_ascii[CONFIG_MANAGER_CAN_HANDSHAKE_MAX_LENGTH];
        char manufacturer[CONFIG_MANAGER_CAN_STRING_MAX_LENGTH];
        char battery_name[CONFIG_MANAGER_CAN_STRING_MAX_LENGTH];
        char battery_family[CONFIG_MANAGER_CAN_STRING_MAX_LENGTH];
        char serial_number[CONFIG_MANAGER_CAN_SERIAL_MAX_LENGTH];
    } identity;
} config_manager_can_settings_internal_t;

/**
 * @brief Context structure for JSON operations
 *
 * This structure is passed to JSON functions to access necessary state
 * from the config_manager without exposing all internal details.
 */
typedef struct {
    // Configuration state
    const config_manager_device_settings_internal_t *device_settings;
    const config_manager_uart_pins_internal_t *uart_pins;
    const config_manager_wifi_settings_internal_t *wifi_settings;
    const config_manager_can_settings_internal_t *can_settings;
    const mqtt_client_config_t *mqtt_config;
    const config_manager_mqtt_topics_t *mqtt_topics;
    uint32_t uart_poll_interval_ms;

    // Snapshot buffers
    char *config_json_full;
    size_t config_json_full_size;
    size_t *config_length_full;
    char *config_json_public;
    size_t config_json_public_size;
    size_t *config_length_public;

    // Event publisher
    event_bus_publish_fn_t event_publisher;

    // Helper function pointers
    const char *(*effective_device_name)(void);
    const char *(*mask_secret)(const char *value);
    void (*parse_mqtt_uri)(const char *uri, char *scheme, size_t scheme_size,
                          char *host, size_t host_size, uint16_t *port);
} config_manager_json_context_t;

// =============================================================================
// JSON Helper Functions
// =============================================================================

/**
 * @brief Get JSON object from parent by field name
 *
 * @param parent Parent JSON object
 * @param field Field name to retrieve
 * @return Pointer to JSON object or NULL if not found or not an object
 */
const cJSON *config_manager_get_object(const cJSON *parent, const char *field);

/**
 * @brief Copy string value from JSON object to destination buffer
 *
 * @param object JSON object containing the field
 * @param field Field name to retrieve
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @return true if string was copied successfully, false otherwise
 */
bool config_manager_copy_json_string(const cJSON *object,
                                     const char *field,
                                     char *dest,
                                     size_t dest_size);

/**
 * @brief Get uint32_t value from JSON object
 *
 * @param object JSON object containing the field
 * @param field Field name to retrieve
 * @param out_value Pointer to store the retrieved value
 * @return true if value was retrieved successfully, false otherwise
 */
bool config_manager_get_uint32_json(const cJSON *object,
                                   const char *field,
                                   uint32_t *out_value);

/**
 * @brief Get int32_t value from JSON object
 *
 * @param object JSON object containing the field
 * @param field Field name to retrieve
 * @param out_value Pointer to store the retrieved value
 * @return true if value was retrieved successfully, false otherwise
 */
bool config_manager_get_int32_json(const cJSON *object,
                                  const char *field,
                                  int32_t *out_value);

// =============================================================================
// Configuration Snapshot Functions
// =============================================================================

/**
 * @brief Build configuration snapshot (both public and full versions)
 *
 * This function generates JSON representations of the current configuration,
 * creating both a public version (with secrets masked) and a full version
 * (with secrets included).
 *
 * @param ctx JSON context with configuration state and buffers
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_build_config_snapshot(config_manager_json_context_t *ctx);

/**
 * @brief Publish configuration snapshot via event bus
 *
 * Publishes the public configuration snapshot (without secrets) to the event bus
 * so other modules can be notified of configuration changes.
 *
 * @param ctx JSON context with event publisher and snapshot
 */
void config_manager_publish_config_snapshot(config_manager_json_context_t *ctx);

// =============================================================================
// Configuration Apply Functions
// =============================================================================

/**
 * @brief Apply configuration from JSON payload
 *
 * Parses a JSON configuration payload and applies the settings to the system.
 * Can optionally persist changes to NVS and/or apply to runtime.
 *
 * @param json JSON string containing configuration
 * @param length Length of JSON string (0 to auto-detect)
 * @param persist If true, persist changes to NVS
 * @param apply_runtime If true, apply changes to runtime immediately
 * @param ctx JSON context with configuration state
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_apply_config_payload(const char *json,
                                             size_t length,
                                             bool persist,
                                             bool apply_runtime,
                                             config_manager_json_context_t *ctx);

// =============================================================================
// File I/O Functions
// =============================================================================

/**
 * @brief Save configuration to /spiffs/config.json
 *
 * Writes the current full configuration snapshot (with secrets) to the
 * SPIFFS filesystem for persistence across reboots.
 *
 * @param ctx JSON context with configuration snapshot
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_save_config_file(config_manager_json_context_t *ctx);

/**
 * @brief Load configuration from /spiffs/config.json
 *
 * Reads configuration from the SPIFFS filesystem and applies it.
 *
 * @param apply_runtime If true, apply changes to runtime immediately
 * @param ctx JSON context with configuration state
 * @return ESP_OK on success, ESP_ERR_NOT_FOUND if file doesn't exist, error code otherwise
 */
esp_err_t config_manager_load_config_file(bool apply_runtime,
                                         config_manager_json_context_t *ctx);

// =============================================================================
// Public API Functions (called from config_manager.c)
// =============================================================================

/**
 * @brief Get configuration as JSON string
 *
 * Returns a JSON representation of the current configuration.
 * Can include or exclude secrets based on flags.
 *
 * This is the implementation for the public API in config_manager.h line 114.
 *
 * @param buffer Output buffer for JSON string
 * @param buffer_size Size of output buffer
 * @param out_length Pointer to store actual length of JSON string
 * @param flags Snapshot flags (include secrets or not)
 * @param ctx JSON context with configuration snapshots
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_get_config_json_impl(char *buffer,
                                              size_t buffer_size,
                                              size_t *out_length,
                                              config_manager_snapshot_flags_t flags,
                                              config_manager_json_context_t *ctx);

/**
 * @brief Set configuration from JSON string
 *
 * Parses and applies configuration from a JSON string.
 * Changes are persisted and applied to runtime.
 *
 * This is the implementation for the public API in config_manager.h line 118.
 *
 * @param json JSON string containing configuration
 * @param length Length of JSON string
 * @param ctx JSON context with configuration state
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_set_config_json_impl(const char *json,
                                              size_t length,
                                              config_manager_json_context_t *ctx);

#ifdef __cplusplus
}
#endif
