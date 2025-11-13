# TinyBMS Register Management Code Extraction Summary

## Overview
Successfully extracted all TinyBMS register management code from `config_manager.c` into dedicated files:
- `/home/user/BMS/main/config_manager/config_manager_registers.h` - Register API declarations
- `/home/user/BMS/main/config_manager/config_manager_registers.c` - Register implementation

## Extracted Components

### 1. Type Definitions (in .h file)
- `config_manager_access_t` - Register access mode (RO/WO/RW)
- `config_manager_value_class_t` - Value classification (NUMERIC/ENUM)
- `config_manager_enum_entry_t` - Enumeration entry structure
- `config_manager_register_descriptor_t` - Complete register descriptor

### 2. Public API Functions (in .h file)
- `config_manager_registers_init()` - Initialize register subsystem
- `config_manager_get_registers_json()` - Serialize register descriptors to JSON
- `config_manager_apply_register_update_json()` - Apply register update from JSON
- `config_manager_load_register_defaults()` - Load default values
- `config_manager_load_persisted_registers()` - Load values from NVS
- `config_manager_get_register_count()` - Get total register count
- `config_manager_registers_initialized()` - Check initialization status
- `config_manager_registers_reset()` - Reset for cleanup/testing

### 3. Implementation Details (in .c file)

#### Static Variables
- `s_register_raw_values[]` - Array storing current raw register values
- `s_registers_initialised` - Initialization flag
- `s_register_events[][]` - Ring buffer for event payloads
- `s_next_register_event` - Next event slot index
- `s_event_publisher` - Event publisher callback
- `s_config_mutex` - Mutex for thread safety

#### Internal Functions
- `config_manager_make_register_key()` - Generate NVS key from address (reg%04X)
- `config_manager_store_register_raw()` - Persist register to NVS
- `config_manager_load_register_raw()` - Load register from NVS
- `config_manager_find_register()` - Find register by key
- `config_manager_raw_to_user()` - Convert raw to user value (apply scale)
- `config_manager_align_raw_value()` - Align value to step/min/max constraints
- `config_manager_convert_user_to_raw()` - Convert user to raw value with validation
- `config_manager_publish_register_change()` - Publish register change event
- `config_manager_json_append()` - Helper for building JSON
- `config_manager_lock()` / `config_manager_unlock()` - Mutex operations
- `config_manager_init_nvs()` - Initialize NVS subsystem

### 4. Generated Definitions Inclusion
- Includes `generated_tiny_rw_registers.inc` for register descriptors
- Contains enum option arrays (e.g., `s_enum_options_307[]`)
- Contains register descriptor array (`s_register_descriptors[]`)
- Contains register count (`s_register_count`)

## Key Features Preserved

### NVS Persistence
- Registers stored with keys like `reg0133`, `reg0307`, etc.
- Automatic save on successful register write
- Validation on load (enum matching, min/max alignment)
- Graceful handling of missing or invalid persisted values

### UART BMS Integration
- `uart_bms_write_register()` called with address and value
- Readback verification from BMS
- Timeout handling (UART_BMS_RESPONSE_TIMEOUT_MS)
- Error propagation to caller

### Validation Pipeline
1. **Enum Validation**: For enum registers, value must match one of the defined options
2. **Scale Validation**: For numeric registers, checks scale > 0
3. **Range Validation**: Enforces min/max constraints if defined
4. **Step Alignment**: Rounds to nearest step if step_raw > 0
5. **Access Control**: Only RW registers can be written

### JSON Serialization (`config_manager_get_registers_json`)
Output format:
```json
{
  "total": 123,
  "registers": [
    {
      "key": "cells_count",
      "label": "Number of Cells",
      "unit": "",
      "group": "Battery",
      "type": "enum",
      "access": "rw",
      "address": 307,
      "scale": 1.0,
      "precision": 0,
      "value": 12,
      "raw": 12,
      "default": 12,
      "comment": "Total battery cells",
      "enum": [
        {"value": 4, "label": "4 cells"},
        {"value": 5, "label": "5 cells"},
        ...
      ]
    },
    ...
  ]
}
```

### JSON Update (`config_manager_apply_register_update_json`)
Input format:
```json
{
  "key": "cells_count",
  "value": 13
}
```

Process:
1. Parse JSON and extract key/value
2. Find register by key
3. Validate and convert user value to raw
4. Write to BMS via UART
5. Store readback value
6. Persist to NVS
7. Publish change event

### Event Publishing
- Publishes `APP_EVENT_ID_CONFIG_UPDATED` events
- Payload format: `{"type":"register_update","key":"...", "value":..., "raw":...}`
- Uses ring buffer to avoid dynamic allocation
- Non-blocking publish with 50ms timeout

## Build System Updates
Updated `/home/user/BMS/main/config_manager/CMakeLists.txt` to include the new source file:
```cmake
idf_component_register(SRCS "config_manager.c"
                            "config_manager_registers.c"
                      INCLUDE_DIRS "."
                      REQUIRES event_bus uart_bms nvs_flash)
```

## Integration Points

### From config_manager.c
The main config manager still needs to:
1. Call `config_manager_registers_init()` during initialization
2. Call `config_manager_load_register_defaults()` if not initialized
3. Call `config_manager_load_persisted_registers()` to restore NVS values
4. Include register count in configuration snapshots
5. Export the two public API functions:
   - `config_manager_get_registers_json()`
   - `config_manager_apply_register_update_json()`

### Dependencies
- `cJSON` - JSON parsing and generation
- `uart_bms` - BMS communication via UART
- `event_bus` - Event publishing
- `nvs_flash` / `nvs` - NVS persistence (ESP32 only)
- `esp_log` - Logging
- `freertos` - Mutex operations

## Thread Safety
- Uses mutex (`s_config_mutex`) passed during initialization
- Lock acquired with portMAX_DELAY for read operations
- Lock acquired with CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS (1000ms) for write operations
- Automatic unlock on error paths (goto cleanup pattern)

## Platform Compatibility
- Full functionality on ESP32 (ESP_PLATFORM defined)
- Stub implementations for non-ESP platforms:
  - `config_manager_store_register_raw()` returns ESP_OK
  - `config_manager_load_register_raw()` returns false
  - NVS operations disabled

## Error Handling
All functions return `esp_err_t` with appropriate codes:
- `ESP_OK` - Success
- `ESP_ERR_INVALID_ARG` - Invalid parameters or JSON
- `ESP_ERR_INVALID_SIZE` - Buffer too small or payload too large
- `ESP_ERR_INVALID_STATE` - Not initialized or invalid register state
- `ESP_ERR_NOT_FOUND` - Register key not found
- `ESP_ERR_TIMEOUT` - Mutex acquisition timeout
- Plus UART and NVS error codes

## Constants and Limits
- `CONFIG_MANAGER_REGISTER_EVENT_BUFFERS` = 4 (ring buffer size)
- `CONFIG_MANAGER_MAX_UPDATE_PAYLOAD` = 192 bytes (event payload)
- `CONFIG_MANAGER_MAX_REGISTER_KEY` = 32 chars (register key length)
- `CONFIG_MANAGER_NAMESPACE` = "gateway_cfg" (NVS namespace)
- `CONFIG_MANAGER_REGISTER_KEY_PREFIX` = "reg" (NVS key prefix)
- `CONFIG_MANAGER_REGISTER_KEY_MAX` = 16 chars (NVS key max)
- `CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS` = 1000ms

## Testing Recommendations
1. Verify register defaults loaded correctly
2. Test NVS persistence across reboots
3. Validate enum constraint enforcement
4. Validate min/max/step constraints
5. Test UART write failures and recovery
6. Verify JSON serialization completeness
7. Test concurrent access with mutex
8. Verify event publishing
9. Test memory limits (large register counts)
10. Platform compatibility (ESP32 vs non-ESP)

## Next Steps
To complete the integration:
1. Update `config_manager.c` to call `config_manager_registers_init()`
2. Remove extracted code from `config_manager.c`
3. Update includes in `config_manager.c` to reference new header
4. Compile and test
5. Verify all register operations work as before
