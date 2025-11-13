# Integration Guide for Extracted Register Management Code

## Files Created
1. `/home/user/BMS/main/config_manager/config_manager_registers.h` (162 lines)
2. `/home/user/BMS/main/config_manager/config_manager_registers.c` (681 lines)
3. `/home/user/BMS/main/config_manager/CMakeLists.txt` (updated)

## Required Changes to config_manager.c

### 1. Add Include
At the top of `config_manager.c`, after existing includes:
```c
#include "config_manager_registers.h"
```

### 2. Remove Extracted Type Definitions (lines ~307-343)
Delete these type definitions (now in config_manager_registers.h):
- `config_manager_access_t` enum
- `config_manager_value_class_t` enum
- `config_manager_enum_entry_t` struct
- `config_manager_register_descriptor_t` struct

### 3. Remove Extracted Static Variables (lines ~904-907)
Delete these static variables (now in config_manager_registers.c):
```c
static uint16_t s_register_raw_values[s_register_count];
static bool s_registers_initialised = false;
static char s_register_events[CONFIG_MANAGER_REGISTER_EVENT_BUFFERS][CONFIG_MANAGER_MAX_UPDATE_PAYLOAD];
static size_t s_next_register_event = 0;
```

### 4. Remove Generated Include (line ~345)
Delete this line (now in config_manager_registers.c):
```c
#include "generated_tiny_rw_registers.inc"
```

### 5. Remove Static Helper Functions (lines ~151-159, 1268-1347, 2016-2136)
Delete these functions (now in config_manager_registers.c):
- `config_manager_make_register_key()` (line ~151)
- `config_manager_store_register_raw()` (line ~1268, ESP_PLATFORM version)
- `config_manager_load_register_raw()` (line ~1292, ESP_PLATFORM version)
- `config_manager_store_register_raw()` (line ~1334, non-ESP stub)
- `config_manager_load_register_raw()` (line ~1341, non-ESP stub)
- `config_manager_find_register()` (line ~2016)
- `config_manager_raw_to_user()` (line ~2034)
- `config_manager_align_raw_value()` (line ~2042)
- `config_manager_convert_user_to_raw()` (line ~2073)

### 6. Remove Static Event Publisher Function (lines ~1374-1410)
Delete this function (now in config_manager_registers.c):
- `config_manager_publish_register_change()`

### 7. Remove Static Load Function (lines ~1711-1717)
Delete this function (now in config_manager_registers.c):
- `config_manager_load_register_defaults()`

### 8. Remove Public API Functions (lines ~2477-2731)
Delete these functions (now in config_manager_registers.c):
- `config_manager_get_registers_json()` (line ~2477)
- `config_manager_apply_register_update_json()` (line ~2635)

### 9. Update Initialization in config_manager_ensure_initialised()
Around line ~2138-2160, modify the function:

**Before:**
```c
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
```

**After:**
```c
static void config_manager_ensure_initialised(void)
{
    // Initialize mutex on first call (thread-safe in FreeRTOS)
    if (s_config_mutex == NULL) {
        s_config_mutex = xSemaphoreCreateMutex();
        if (s_config_mutex == NULL) {
            ESP_LOGE(TAG, "Failed to create config mutex");
        }
        // Initialize register subsystem with mutex
        config_manager_registers_init(s_event_publisher, s_config_mutex);
    }

    if (!config_manager_registers_initialized()) {
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
```

### 10. Update config_manager_load_persistent_settings()
Around line ~1039-1064, update the register loading section:

**Before:**
```c
    for (size_t i = 0; i < s_register_count; ++i) {
        const config_manager_register_descriptor_t *desc = &s_register_descriptors[i];
        uint16_t stored_raw = 0;
        if (!config_manager_load_register_raw(desc->address, &stored_raw)) {
            continue;
        }

        if (desc->value_class == CONFIG_MANAGER_VALUE_ENUM) {
            bool found = false;
            for (size_t e = 0; e < desc->enum_count; ++e) {
                if (desc->enum_values[e].value == stored_raw) {
                    found = true;
                    break;
                }
            }
            if (found) {
                s_register_raw_values[i] = stored_raw;
            }
            continue;
        }

        uint16_t aligned = 0;
        if (config_manager_align_raw_value(desc, (float)stored_raw, &aligned) == ESP_OK) {
            s_register_raw_values[i] = aligned;
        }
    }
```

**After:**
```c
    // Load persisted register values from NVS
    config_manager_load_persisted_registers();
```

### 11. Update config_manager_render_config_snapshot_locked()
Around line ~1454, update to use new API:

**Before:**
```c
    CHECK_JSON(cJSON_AddNumberToObject(root, "register_count", (double)s_register_count));
```

**After:**
```c
    CHECK_JSON(cJSON_AddNumberToObject(root, "register_count", (double)config_manager_get_register_count()));
```

### 12. Update config_manager_deinit()
Around line ~2760-2770, update to call reset function:

**Before:**
```c
    s_registers_initialised = false;
    // ...
    memset(s_register_raw_values, 0, sizeof(s_register_raw_values));
    memset(s_register_events, 0, sizeof(s_register_events));
```

**After:**
```c
    config_manager_registers_reset();
```

### 13. Remove Related Constants (lines ~36-42)
These constants should be removed as they're now in config_manager_registers.c:
```c
#define CONFIG_MANAGER_REGISTER_EVENT_BUFFERS 4
#define CONFIG_MANAGER_MAX_UPDATE_PAYLOAD     192
#define CONFIG_MANAGER_MAX_REGISTER_KEY       32
#define CONFIG_MANAGER_REGISTER_KEY_PREFIX    "reg"
#define CONFIG_MANAGER_REGISTER_KEY_MAX       16
```

Keep only:
```c
#define CONFIG_MANAGER_NAMESPACE              "gateway_cfg"
```
(since it's also used for MQTT and other config)

## Verification Steps

### 1. Compilation Check
```bash
cd /home/user/BMS
idf.py build
```

### 2. Size Comparison
Check that the binary size hasn't changed significantly:
```bash
idf.py size
```

### 3. Functional Tests
- Test register read via JSON API
- Test register write via JSON API
- Test NVS persistence across reboots
- Verify register defaults are loaded
- Test enum validation
- Test min/max validation
- Test UART communication

### 4. Code Review Checklist
- [ ] No duplicate definitions between files
- [ ] All register functions accessible via new API
- [ ] Mutex properly passed and used
- [ ] Event publisher properly passed
- [ ] NVS operations work correctly
- [ ] Generated include only appears once
- [ ] No dangling references to removed functions

## Expected Benefits

### Code Organization
- **Separation of Concerns**: Register management isolated from general config
- **Modularity**: Can reuse register code in other projects
- **Maintainability**: Easier to find and modify register-specific code

### Compile Time
- **Smaller Files**: config_manager.c reduced by ~700 lines
- **Parallel Builds**: Register code can compile independently

### Testing
- **Unit Testing**: Register functions can be tested in isolation
- **Mocking**: Easier to mock register subsystem for config tests

### Documentation
- **API Clarity**: Clear interface in config_manager_registers.h
- **Encapsulation**: Internal helpers hidden from config_manager.c

## Rollback Plan
If issues arise:
1. Revert CMakeLists.txt change
2. Delete new files:
   - `config_manager_registers.h`
   - `config_manager_registers.c`
3. Restore original config_manager.c from git

## File Statistics

### Before Extraction
- `config_manager.c`: ~2782 lines
- Total register code: ~700 lines (25% of file)

### After Extraction
- `config_manager.c`: ~2082 lines (estimated after cleanup)
- `config_manager_registers.c`: 681 lines
- `config_manager_registers.h`: 162 lines
- Total: Similar, but better organized

## Contact
For questions or issues with the integration, refer to:
- `REGISTER_EXTRACTION_SUMMARY.md` for detailed component list
- Git history for original code location
- Code comments in extracted files
