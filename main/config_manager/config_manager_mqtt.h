#pragma once

/**
 * @file config_manager_mqtt.h
 * @brief MQTT configuration management module
 *
 * Handles MQTT URI parsing, configuration validation, topic management,
 * and MQTT client configuration getters/setters.
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"
#include "config_manager.h"
#include "mqtt_client.h"

/**
 * @brief Parse MQTT URI into components
 *
 * Extracts scheme (mqtt/mqtts), hostname, and port from a URI string.
 * Supports formats: mqtt://host:port, mqtts://host:port
 *
 * @param uri Input URI string
 * @param out_scheme Output buffer for scheme (e.g., "mqtt" or "mqtts")
 * @param scheme_size Size of scheme buffer
 * @param out_host Output buffer for hostname
 * @param host_size Size of hostname buffer
 * @param out_port Output pointer for port number
 */
void config_manager_parse_mqtt_uri(const char *uri,
                                    char *out_scheme,
                                    size_t scheme_size,
                                    char *out_host,
                                    size_t host_size,
                                    uint16_t *out_port);

/**
 * @brief Validate and sanitize MQTT configuration
 *
 * Ensures MQTT config has valid values, sets defaults for missing fields.
 *
 * @param config MQTT configuration to sanitize (modified in place)
 */
void config_manager_sanitise_mqtt_config(mqtt_client_config_t *config);

/**
 * @brief Safe string copy with null termination
 *
 * Copies source string to destination with guaranteed null termination.
 * Handles NULL pointers safely.
 *
 * @param dest Destination buffer
 * @param dest_size Size of destination buffer
 * @param src Source string (can be NULL)
 */
void config_manager_copy_string(char *dest, size_t dest_size, const char *src);

/**
 * @brief Copy MQTT topics structure
 *
 * Safely copies all topic strings from source to destination.
 *
 * @param dest Destination topics structure
 * @param src Source topics structure
 */
void config_manager_copy_topics(config_manager_mqtt_topics_t *dest,
                                const config_manager_mqtt_topics_t *src);

/**
 * @brief Reset MQTT topics to default values
 *
 * Generates default topic names based on current device name.
 */
void config_manager_reset_mqtt_topics(void);

/**
 * @brief Sanitize MQTT topics
 *
 * Ensures all topic strings are properly null-terminated.
 *
 * @param topics Topics structure to sanitize
 */
void config_manager_sanitise_mqtt_topics(config_manager_mqtt_topics_t *topics);

/**
 * @brief Ensure MQTT topics are loaded
 *
 * Lazy initialization of topics if not already loaded.
 */
void config_manager_ensure_topics_loaded(void);

/**
 * @brief Generate default MQTT topics for a device name
 *
 * Creates standard topic paths using the provided device name.
 *
 * @param device_name Device name to use (or NULL for default)
 * @param topics Output topics structure
 */
void config_manager_make_default_topics_for_name(const char *device_name,
                                                  config_manager_mqtt_topics_t *topics);

/**
 * @brief Update topics when device name changes
 *
 * If current topics match old default topics, updates them to new defaults.
 *
 * @param old_name Previous device name
 * @param new_name New device name
 */
void config_manager_update_topics_for_device_change(const char *old_name, const char *new_name);

/**
 * @brief Convert string to lowercase
 *
 * Modifies string in place to lowercase all characters.
 *
 * @param value String to convert (modified in place)
 */
void config_manager_lowercase(char *value);

/**
 * @brief Get default port for MQTT scheme
 *
 * Returns 8883 for "mqtts", 1883 for "mqtt" or other.
 *
 * @param scheme MQTT scheme string
 * @return Default port number
 */
uint16_t config_manager_default_port_for_scheme(const char *scheme);

/**
 * @brief Get MQTT client configuration
 *
 * Returns a thread-safe snapshot of current MQTT configuration.
 *
 * @return Pointer to MQTT client configuration (read-only snapshot)
 */
const mqtt_client_config_t *config_manager_get_mqtt_client_config(void);

/**
 * @brief Set MQTT client configuration
 *
 * Updates and persists MQTT configuration to NVS.
 * Thread-safe operation.
 *
 * @param config New MQTT configuration
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_set_mqtt_client_config(const mqtt_client_config_t *config);

/**
 * @brief Get MQTT topics
 *
 * Returns a thread-safe snapshot of current MQTT topics.
 *
 * @return Pointer to MQTT topics structure (read-only snapshot)
 */
const config_manager_mqtt_topics_t *config_manager_get_mqtt_topics(void);

/**
 * @brief Set MQTT topics
 *
 * Updates and persists MQTT topics to NVS.
 * Thread-safe operation.
 *
 * @param topics New MQTT topics configuration
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_set_mqtt_topics(const config_manager_mqtt_topics_t *topics);
