#pragma once

/**
 * @file config_manager_nvs.h
 * @brief NVS persistence interface for configuration manager
 *
 * Provides functions for loading and storing configuration data
 * to/from ESP32's Non-Volatile Storage (NVS).
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"
#include "mqtt_client.h"
#include "config_manager.h"

/**
 * @brief Initialize NVS flash subsystem
 *
 * Must be called before any NVS operations. Handles NVS initialization
 * and automatic erase/retry if necessary.
 *
 * @return ESP_OK on success, error code otherwise
 */
#ifdef ESP_PLATFORM
esp_err_t config_manager_init_nvs(void);
#endif

/**
 * @brief Load all persistent settings from NVS
 *
 * Loads UART poll interval, MQTT configuration, MQTT topics, and
 * register values from NVS. Also loads configuration file from SPIFFS.
 * This function is idempotent - subsequent calls are no-ops.
 */
void config_manager_load_persistent_settings(void);

/**
 * @brief Store UART poll interval to NVS
 *
 * @param interval_ms Poll interval in milliseconds
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_store_poll_interval(uint32_t interval_ms);

/**
 * @brief Store MQTT client configuration to NVS
 *
 * Persists broker URI, credentials, TLS settings, QoS, and keepalive
 * parameters to NVS. All settings are stored atomically.
 *
 * @param config MQTT client configuration to store
 * @return ESP_OK on success, ESP_ERR_INVALID_ARG if config is NULL
 */
esp_err_t config_manager_store_mqtt_config_to_nvs(const mqtt_client_config_t *config);

/**
 * @brief Store MQTT topics configuration to NVS
 *
 * Persists all MQTT topic strings (status, metrics, config, CAN raw,
 * CAN decoded, CAN ready) to NVS.
 *
 * @param topics MQTT topics configuration to store
 * @return ESP_OK on success, ESP_ERR_INVALID_ARG if topics is NULL
 */
esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics);

/**
 * @brief Store WiFi AP secret to NVS
 *
 * Persists the WiFi Access Point secret/password to NVS for use across reboots.
 *
 * @param secret WiFi AP secret string to store
 */
void config_manager_store_ap_secret_to_nvs(const char *secret);

/**
 * @brief Ensure WiFi AP secret is loaded from NVS
 *
 * Loads the WiFi AP secret from NVS if not already loaded. If no secret
 * exists in NVS or it's too short, generates a new random secret and
 * stores it to NVS. This function is idempotent.
 */
void config_manager_ensure_ap_secret_loaded(void);

/**
 * @brief Format a register NVS key from address
 *
 * Creates a key string like "regXXXX" where XXXX is the hex address.
 *
 * @param address Register address
 * @param out_key Output buffer for key string
 * @param out_size Size of output buffer
 */
void config_manager_make_register_key(uint16_t address, char *out_key, size_t out_size);

/**
 * @brief Store a register raw value to NVS
 *
 * @param address Register address
 * @param raw_value Raw register value
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_store_register_raw(uint16_t address, uint16_t raw_value);

/**
 * @brief Load a register raw value from NVS
 *
 * @param address Register address
 * @param out_value Pointer to store loaded value
 * @return true if value was loaded successfully, false otherwise
 */
bool config_manager_load_register_raw(uint16_t address, uint16_t *out_value);
