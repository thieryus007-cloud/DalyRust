#pragma once

/**
 * @file config_manager_registers.h
 * @brief TinyBMS register management API
 *
 * Provides functionality for managing TinyBMS registers including:
 * - Serialization of register descriptors to JSON
 * - Application of register updates from JSON
 * - Register read/write via uart_bms
 * - Register NVS persistence (reg%04X keys)
 * - Scale/min/max/enum validation
 */

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "esp_err.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include "event_bus.h"

#ifdef __cplusplus
extern "C" {
#endif

/**
 * @brief Register access mode
 */
typedef enum {
    CONFIG_MANAGER_ACCESS_RO = 0,  ///< Read-only
    CONFIG_MANAGER_ACCESS_WO,      ///< Write-only
    CONFIG_MANAGER_ACCESS_RW,      ///< Read-write
} config_manager_access_t;

/**
 * @brief Register value classification
 */
typedef enum {
    CONFIG_MANAGER_VALUE_NUMERIC = 0,  ///< Numeric value with scaling
    CONFIG_MANAGER_VALUE_ENUM,         ///< Enumerated value
} config_manager_value_class_t;

/**
 * @brief Enumeration entry for register values
 */
typedef struct {
    uint16_t value;      ///< Raw enumeration value
    const char *label;   ///< Human-readable label
} config_manager_enum_entry_t;

/**
 * @brief Complete descriptor for a TinyBMS register
 */
typedef struct {
    uint16_t address;                           ///< Register address
    const char *key;                            ///< Unique key identifier
    const char *label;                          ///< Human-readable label
    const char *unit;                           ///< Unit of measurement
    const char *group;                          ///< Grouping category
    const char *comment;                        ///< Descriptive comment
    const char *type;                           ///< Type description
    config_manager_access_t access;             ///< Access mode
    float scale;                                ///< Scaling factor (raw to user)
    uint8_t precision;                          ///< Decimal precision
    bool has_min;                               ///< Whether minimum is defined
    uint16_t min_raw;                           ///< Minimum raw value
    bool has_max;                               ///< Whether maximum is defined
    uint16_t max_raw;                           ///< Maximum raw value
    float step_raw;                             ///< Step size in raw units
    uint16_t default_raw;                       ///< Default raw value
    config_manager_value_class_t value_class;   ///< Value classification
    const config_manager_enum_entry_t *enum_values;  ///< Enum options (if applicable)
    size_t enum_count;                          ///< Number of enum options
} config_manager_register_descriptor_t;

/**
 * @brief Initialize the register management subsystem
 *
 * This function must be called before any other register functions.
 * It loads register defaults and restores persisted values from NVS.
 *
 * @param event_publisher Event publisher callback for change notifications
 * @param config_mutex Mutex for thread-safe access to configuration state
 */
void config_manager_registers_init(event_bus_publish_fn_t event_publisher,
                                   SemaphoreHandle_t config_mutex);

/**
 * @brief Serialize all register descriptors to JSON
 *
 * Generates a JSON document containing all register descriptors with their
 * current values, metadata, and constraints.
 *
 * @param[out] buffer Buffer to receive JSON string
 * @param[in] buffer_size Size of output buffer
 * @param[out] out_length Actual length of JSON string (optional)
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_get_registers_json(char *buffer,
                                            size_t buffer_size,
                                            size_t *out_length);

/**
 * @brief Apply a register update from JSON
 *
 * Parses JSON containing a register key and value, validates the value,
 * writes it to the BMS via UART, and persists it to NVS.
 *
 * Expected JSON format:
 * {
 *   "key": "register_key",
 *   "value": 123.45
 * }
 *
 * @param[in] json JSON payload containing register update
 * @param[in] length Length of JSON payload (0 for null-terminated)
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t config_manager_apply_register_update_json(const char *json,
                                                    size_t length);

/**
 * @brief Load register defaults
 *
 * Initializes all registers to their default values as defined in the
 * register descriptor table.
 */
void config_manager_load_register_defaults(void);

/**
 * @brief Load persisted register values from NVS
 *
 * Restores register values that were previously saved to NVS.
 * Values are validated against enum options or min/max constraints.
 */
void config_manager_load_persisted_registers(void);

/**
 * @brief Get the total number of registers
 *
 * @return Number of defined registers
 */
size_t config_manager_get_register_count(void);

/**
 * @brief Check if registers have been initialized
 *
 * @return true if initialized, false otherwise
 */
bool config_manager_registers_initialized(void);

/**
 * @brief Reset register initialization state (for testing/cleanup)
 */
void config_manager_registers_reset(void);

#ifdef __cplusplus
}
#endif
