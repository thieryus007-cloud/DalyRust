# Configuration Manager JSON Extraction Summary

## Overview
This document describes the extraction of JSON serialization/deserialization code from `config_manager.c` into separate module files `config_manager_json.c` and `config_manager_json.h`.

## Files Created

### 1. `/home/user/BMS/main/config_manager/config_manager_json.h`
Header file containing:
- Public API declarations for JSON operations
- Helper function prototypes for JSON parsing
- Context structure `config_manager_json_context_t` for passing state
- Type definitions for internal configuration structures

### 2. `/home/user/BMS/main/config_manager/config_manager_json.c`
Implementation file containing:
- All JSON helper functions (get_object, copy_json_string, get_uint32_json, get_int32_json)
- Configuration snapshot building and rendering
- Configuration snapshot publishing
- JSON payload parsing and application
- File I/O for /spiffs/config.json
- Public API implementations

## Extracted Functions

### JSON Helper Functions (lines 468-544 from original)
- `config_manager_get_object()` - Get JSON object from parent
- `config_manager_copy_json_string()` - Copy string from JSON
- `config_manager_get_uint32_json()` - Get uint32 from JSON
- `config_manager_get_int32_json()` - Get int32 from JSON

### Configuration Snapshot Functions (lines 1412-1709 from original)
- `config_manager_render_config_snapshot()` - Internal function to render JSON
- `config_manager_build_config_snapshot()` - Build both public and full snapshots
- `config_manager_build_config_snapshot_locked()` - Version requiring external lock

### Event Publishing (lines 1357-1370 from original)
- `config_manager_publish_config_snapshot()` - Publish config update event

### Configuration Apply Functions (lines 1719-2014 from original)
- `config_manager_apply_config_payload()` - Parse and apply JSON configuration

### File I/O Functions (lines 1096-1183 from original)
- `config_manager_mount_spiffs()` - Mount SPIFFS filesystem
- `config_manager_save_config_file()` - Save config to /spiffs/config.json
- `config_manager_load_config_file()` - Load config from /spiffs/config.json

### Public API Implementations (lines 2365-2408 from original)
- `config_manager_get_config_json_impl()` - Get config as JSON string
- `config_manager_set_config_json_impl()` - Set config from JSON string

## Integration Requirements

### 1. Modify config_manager.c

The following changes need to be made to `config_manager.c`:

#### Add Include
```c
#include "config_manager_json.h"
```

#### Create Context Initialization Function
```c
static void config_manager_init_json_context(config_manager_json_context_t *ctx)
{
    ctx->device_settings = (const config_manager_device_settings_internal_t *)&s_device_settings;
    ctx->uart_pins = (const config_manager_uart_pins_internal_t *)&s_uart_pins;
    ctx->wifi_settings = (const config_manager_wifi_settings_internal_t *)&s_wifi_settings;
    ctx->can_settings = (const config_manager_can_settings_internal_t *)&s_can_settings;
    ctx->mqtt_config = &s_mqtt_config;
    ctx->mqtt_topics = &s_mqtt_topics;
    ctx->uart_poll_interval_ms = s_uart_poll_interval_ms;

    ctx->config_json_full = s_config_json_full;
    ctx->config_json_full_size = sizeof(s_config_json_full);
    ctx->config_length_full = &s_config_length_full;
    ctx->config_json_public = s_config_json_public;
    ctx->config_json_public_size = sizeof(s_config_json_public);
    ctx->config_length_public = &s_config_length_public;

    ctx->event_publisher = s_event_publisher;
    ctx->effective_device_name = config_manager_effective_device_name;
    ctx->mask_secret = config_manager_mask_secret;
    ctx->parse_mqtt_uri = config_manager_parse_mqtt_uri;
}
```

#### Update Public API Functions
Replace the implementations of `config_manager_get_config_json()` and `config_manager_set_config_json()` with calls to the JSON module:

```c
esp_err_t config_manager_get_config_json(char *buffer,
                                         size_t buffer_size,
                                         size_t *out_length,
                                         config_manager_snapshot_flags_t flags)
{
    if (buffer == NULL || buffer_size == 0) {
        return ESP_ERR_INVALID_ARG;
    }

    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    config_manager_json_context_t ctx;
    config_manager_init_json_context(&ctx);

    esp_err_t result = config_manager_get_config_json_impl(buffer, buffer_size,
                                                           out_length, flags, &ctx);
    config_manager_unlock();
    return result;
}

esp_err_t config_manager_set_config_json(const char *json, size_t length)
{
    if (json == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    config_manager_ensure_initialised();

    config_manager_json_context_t ctx;
    config_manager_init_json_context(&ctx);

    return config_manager_set_config_json_impl(json, length, &ctx);
}
```

#### Remove Old Functions
Delete the following functions from config_manager.c (they're now in config_manager_json.c):
- Lines 468-544: `config_manager_get_object()`, `config_manager_copy_json_string()`,
  `config_manager_get_uint32_json()`, `config_manager_get_int32_json()`
- Lines 1096-1120: `config_manager_mount_spiffs()` (ESP_PLATFORM version)
- Lines 1122-1183: `config_manager_save_config_file()`, `config_manager_load_config_file()`
- Lines 1349-1355: `config_manager_select_secret_value()` (now internal to JSON module)
- Lines 1357-1370: `config_manager_publish_config_snapshot()`
- Lines 1412-1709: `config_manager_render_config_snapshot_locked()`,
  `config_manager_build_config_snapshot_locked()`, `config_manager_build_config_snapshot()`
- Lines 1719-2014: `config_manager_apply_config_payload()`

#### Expose Required Helper Functions
Some helper functions need to be made non-static so the JSON module can call them:

```c
// Remove 'static' keyword from these functions:
uint32_t config_manager_clamp_poll_interval(uint32_t interval_ms);
void config_manager_update_topics_for_device_change(const char *old_name, const char *new_name);
void config_manager_sanitise_mqtt_config(mqtt_client_config_t *config);
void config_manager_sanitise_mqtt_topics(config_manager_mqtt_topics_t *topics);
void config_manager_apply_ap_secret_if_needed(config_manager_wifi_settings_internal_t *wifi);
esp_err_t config_manager_store_poll_interval(uint32_t interval_ms);
esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics);
```

### 2. Update Build System

Add the new source file to CMakeLists.txt or the appropriate build configuration:

```cmake
# In the component's CMakeLists.txt, add to SRCS:
set(SRCS
    "config_manager.c"
    "config_manager_json.c"
    # ... other sources
)
```

### 3. Known Limitations

#### Incomplete Implementation in config_manager_apply_config_payload()
The `config_manager_apply_config_payload()` function in the JSON module can parse the JSON but cannot fully apply the settings because:
1. It doesn't have access to the mutex-protected internal state
2. It cannot call functions like `uart_bms_set_poll_interval_ms()`
3. It cannot trigger WiFi restart for STA credential changes

**Solution**: This function needs to be refactored to either:
- Return the parsed settings to the caller (config_manager.c) for application
- Accept callback functions for state modification
- Be moved back to config_manager.c with only JSON parsing extracted

The current implementation returns `ESP_ERR_NOT_SUPPORTED` with a warning message.

## Benefits of Extraction

1. **Separation of Concerns**: JSON parsing logic is isolated from configuration management
2. **Testability**: JSON functions can be tested independently
3. **Maintainability**: Easier to locate and modify JSON-related code
4. **Reusability**: JSON utilities can be used by other modules if needed
5. **Reduced File Size**: config_manager.c is now ~800 lines smaller

## File Sizes

- **config_manager_json.h**: ~250 lines
- **config_manager_json.c**: ~950 lines
- **Total extracted**: ~1200 lines of JSON-related code

## Testing Checklist

After integration, verify:
- [ ] Configuration can be read via `config_manager_get_config_json()`
- [ ] Configuration can be updated via `config_manager_set_config_json()`
- [ ] Config snapshots are built correctly on changes
- [ ] Config snapshots are published to event bus
- [ ] Config file is saved to /spiffs/config.json
- [ ] Config file is loaded from /spiffs/config.json on boot
- [ ] Secrets are properly masked in public snapshots
- [ ] MQTT topic updates work after device name changes
- [ ] WiFi credentials updates trigger reconnection

## Next Steps

1. Integrate the new files into the build system
2. Modify config_manager.c as described above
3. Refactor `config_manager_apply_config_payload()` to properly handle state updates
4. Test all JSON operations thoroughly
5. Consider creating unit tests for the JSON module
6. Update documentation to reference the new module structure

## References

- Original file: `/home/user/BMS/main/config_manager/config_manager.c`
- Public API: `/home/user/BMS/main/config_manager/config_manager.h` (lines 114, 118)
- Event definitions: `app_events.h`
- MQTT topic templates: `mqtt_topics.h`
