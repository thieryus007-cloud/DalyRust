#include "config_manager_registers.h"

#include <math.h>
#include <stdarg.h>
#include <stdio.h>
#include <string.h>

#include "cJSON.h"

#include "esp_log.h"

#include "uart_bms.h"
#include "app_events.h"

#ifdef ESP_PLATFORM
#include "nvs_flash.h"
#include "nvs.h"
#endif

#define CONFIG_MANAGER_REGISTER_EVENT_BUFFERS 4
#define CONFIG_MANAGER_MAX_UPDATE_PAYLOAD     192
#define CONFIG_MANAGER_MAX_REGISTER_KEY       32
#define CONFIG_MANAGER_NAMESPACE              "gateway_cfg"
#define CONFIG_MANAGER_REGISTER_KEY_PREFIX    "reg"
#define CONFIG_MANAGER_REGISTER_KEY_MAX       16

// Include the generated register definitions
#include "generated_tiny_rw_registers.inc"

static const char *TAG = "config_manager_registers";

// Global state
static uint16_t s_register_raw_values[s_register_count];
static bool s_registers_initialised = false;
static char s_register_events[CONFIG_MANAGER_REGISTER_EVENT_BUFFERS][CONFIG_MANAGER_MAX_UPDATE_PAYLOAD];
static size_t s_next_register_event = 0;
static event_bus_publish_fn_t s_event_publisher = NULL;
static SemaphoreHandle_t s_config_mutex = NULL;
static const TickType_t CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS = pdMS_TO_TICKS(1000);

#ifdef ESP_PLATFORM
static bool s_nvs_initialised = false;

static esp_err_t config_manager_init_nvs(void)
{
    if (s_nvs_initialised) {
        return ESP_OK;
    }

    esp_err_t err = nvs_flash_init();
    if (err == ESP_ERR_NVS_NO_FREE_PAGES || err == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_LOGW(TAG, "Erasing NVS partition due to %s", esp_err_to_name(err));
        esp_err_t erase_err = nvs_flash_erase();
        if (erase_err != ESP_OK) {
            return erase_err;
        }
        err = nvs_flash_init();
    }

    if (err == ESP_OK) {
        s_nvs_initialised = true;
    } else {
        ESP_LOGW(TAG, "Failed to initialise NVS: %s", esp_err_to_name(err));
    }
    return err;
}
#endif

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

static bool config_manager_json_append(char *buffer, size_t buffer_size, size_t *offset, const char *fmt, ...)
{
    if (buffer == NULL || buffer_size == 0 || offset == NULL) {
        return false;
    }

    va_list args;
    va_start(args, fmt);
    int written = vsnprintf(buffer + *offset, buffer_size - *offset, fmt, args);
    va_end(args);

    if (written < 0) {
        return false;
    }

    size_t remaining = buffer_size - *offset;
    if ((size_t)written >= remaining) {
        return false;
    }

    *offset += (size_t)written;
    return true;
}

static void config_manager_make_register_key(uint16_t address, char *out_key, size_t out_size)
{
    if (out_key == NULL || out_size == 0) {
        return;
    }
    if (snprintf(out_key, out_size, CONFIG_MANAGER_REGISTER_KEY_PREFIX "%04X", (unsigned)address) >= (int)out_size) {
        out_key[out_size - 1] = '\0';
    }
}

#ifdef ESP_PLATFORM
static esp_err_t config_manager_store_register_raw(uint16_t address, uint16_t raw_value)
{
    esp_err_t err = config_manager_init_nvs();
    if (err != ESP_OK) {
        return err;
    }

    nvs_handle_t handle = 0;
    err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        return err;
    }

    char key[CONFIG_MANAGER_REGISTER_KEY_MAX];
    config_manager_make_register_key(address, key, sizeof(key));

    err = nvs_set_u16(handle, key, raw_value);
    if (err == ESP_OK) {
        err = nvs_commit(handle);
    }
    nvs_close(handle);
    return err;
}

static bool config_manager_load_register_raw(uint16_t address, uint16_t *out_value)
{
    if (out_value == NULL) {
        return false;
    }

    esp_err_t err = config_manager_init_nvs();
    if (err != ESP_OK) {
        return false;
    }

    nvs_handle_t handle = 0;
    err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READONLY, &handle);
    if (err != ESP_OK) {
        return false;
    }

    char key[CONFIG_MANAGER_REGISTER_KEY_MAX];
    config_manager_make_register_key(address, key, sizeof(key));
    uint16_t value = 0;
    err = nvs_get_u16(handle, key, &value);
    nvs_close(handle);
    if (err != ESP_OK) {
        return false;
    }

    *out_value = value;
    return true;
}
#else
static esp_err_t config_manager_store_register_raw(uint16_t address, uint16_t raw_value)
{
    (void)address;
    (void)raw_value;
    return ESP_OK;
}

static bool config_manager_load_register_raw(uint16_t address, uint16_t *out_value)
{
    (void)address;
    (void)out_value;
    return false;
}
#endif

static bool config_manager_find_register(const char *key, size_t *index_out)
{
    if (key == NULL) {
        return false;
    }

    for (size_t i = 0; i < s_register_count; ++i) {
        if (strcmp(s_register_descriptors[i].key, key) == 0) {
            if (index_out != NULL) {
                *index_out = i;
            }
            return true;
        }
    }

    return false;
}

static float config_manager_raw_to_user(const config_manager_register_descriptor_t *desc, uint16_t raw_value)
{
    if (desc == NULL) {
        return 0.0f;
    }
    return (float)raw_value * desc->scale;
}

static esp_err_t config_manager_align_raw_value(const config_manager_register_descriptor_t *desc,
                                                float requested_raw,
                                                uint16_t *out_raw)
{
    if (desc == NULL || out_raw == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    float aligned_raw = requested_raw;
    if (desc->step_raw > 0.0f) {
        float base = desc->has_min ? (float)desc->min_raw : 0.0f;
        float steps = (aligned_raw - base) / desc->step_raw;
        float rounded = nearbyintf(steps);
        aligned_raw = base + desc->step_raw * rounded;
    }

    if (desc->has_min && aligned_raw < (float)desc->min_raw) {
        aligned_raw = (float)desc->min_raw;
    }
    if (desc->has_max && aligned_raw > (float)desc->max_raw) {
        aligned_raw = (float)desc->max_raw;
    }

    if (aligned_raw < 0.0f || aligned_raw > 65535.0f) {
        return ESP_ERR_INVALID_ARG;
    }

    *out_raw = (uint16_t)lrintf(aligned_raw);
    return ESP_OK;
}

static esp_err_t config_manager_convert_user_to_raw(const config_manager_register_descriptor_t *desc,
                                                    float user_value,
                                                    uint16_t *out_raw,
                                                    float *out_aligned_user)
{
    if (desc == NULL || out_raw == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    if (desc->access != CONFIG_MANAGER_ACCESS_RW) {
        return ESP_ERR_INVALID_STATE;
    }

    if (desc->value_class == CONFIG_MANAGER_VALUE_ENUM) {
        uint16_t candidate = (uint16_t)lrintf(user_value);
        for (size_t i = 0; i < desc->enum_count; ++i) {
            if (desc->enum_values[i].value == candidate) {
                *out_raw = candidate;
                if (out_aligned_user != NULL) {
                    *out_aligned_user = (float)candidate;
                }
                return ESP_OK;
            }
        }
        ESP_LOGW(TAG, "%s value %.3f does not match enum options", desc->key, user_value);
        return ESP_ERR_INVALID_ARG;
    }

    if (desc->scale <= 0.0f) {
        ESP_LOGW(TAG, "Register %s has invalid scale %.3f", desc->key, desc->scale);
        return ESP_ERR_INVALID_STATE;
    }

    float requested_raw = user_value / desc->scale;
    uint16_t raw_value = 0;
    esp_err_t err = config_manager_align_raw_value(desc, requested_raw, &raw_value);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "%s unable to align %.3f -> raw", desc->key, user_value);
        return err;
    }

    if (desc->has_min && raw_value < desc->min_raw) {
        ESP_LOGW(TAG,
                 "%s raw %u below minimum %u",
                 desc->key,
                 (unsigned)raw_value,
                 (unsigned)desc->min_raw);
        return ESP_ERR_INVALID_ARG;
    }
    if (desc->has_max && raw_value > desc->max_raw) {
        ESP_LOGW(TAG,
                 "%s raw %u above maximum %u",
                 desc->key,
                 (unsigned)raw_value,
                 (unsigned)desc->max_raw);
        return ESP_ERR_INVALID_ARG;
    }

    *out_raw = raw_value;
    if (out_aligned_user != NULL) {
        *out_aligned_user = config_manager_raw_to_user(desc, raw_value);
    }
    return ESP_OK;
}

static void config_manager_publish_register_change(const config_manager_register_descriptor_t *desc,
                                                   uint16_t raw_value)
{
    if (s_event_publisher == NULL || desc == NULL) {
        return;
    }

    size_t slot = s_next_register_event;
    s_next_register_event = (s_next_register_event + 1) % CONFIG_MANAGER_REGISTER_EVENT_BUFFERS;

    char *payload = s_register_events[slot];
    float user_value = (desc->value_class == CONFIG_MANAGER_VALUE_ENUM)
                           ? (float)raw_value
                           : config_manager_raw_to_user(desc, raw_value);
    int precision = (desc->value_class == CONFIG_MANAGER_VALUE_ENUM) ? 0 : desc->precision;
    int written = snprintf(payload,
                           CONFIG_MANAGER_MAX_UPDATE_PAYLOAD,
                           "{\"type\":\"register_update\",\"key\":\"%s\",\"value\":%.*f,\"raw\":%u}",
                           desc->key,
                           precision,
                           user_value,
                           (unsigned)raw_value);
    if (written < 0 || written >= CONFIG_MANAGER_MAX_UPDATE_PAYLOAD) {
        ESP_LOGW(TAG, "Register update payload truncated for %s", desc->key);
        return;
    }

    event_bus_event_t event = {
        .id = APP_EVENT_ID_CONFIG_UPDATED,
        .payload = payload,
        .payload_size = (size_t)written + 1,
    };

    if (!s_event_publisher(&event, pdMS_TO_TICKS(50))) {
        ESP_LOGW(TAG, "Failed to publish register update for %s", desc->key);
    }
}

void config_manager_registers_init(event_bus_publish_fn_t event_publisher,
                                   SemaphoreHandle_t config_mutex)
{
    s_event_publisher = event_publisher;
    s_config_mutex = config_mutex;
}

void config_manager_load_register_defaults(void)
{
    for (size_t i = 0; i < s_register_count; ++i) {
        s_register_raw_values[i] = s_register_descriptors[i].default_raw;
    }
    s_registers_initialised = true;
}

void config_manager_load_persisted_registers(void)
{
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
}

esp_err_t config_manager_get_registers_json(char *buffer, size_t buffer_size, size_t *out_length)
{
    if (buffer == NULL || buffer_size == 0) {
        return ESP_ERR_INVALID_ARG;
    }

    if (!s_registers_initialised) {
        ESP_LOGW(TAG, "Registers not initialized");
        return ESP_ERR_INVALID_STATE;
    }

    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    esp_err_t result = ESP_OK;
    size_t offset = 0;

    if (!config_manager_json_append(buffer,
                                    buffer_size,
                                    &offset,
                                    "{\"total\":%zu,\"registers\":[",
                                    s_register_count)) {
        result = ESP_ERR_INVALID_SIZE;
        goto cleanup;
    }

    for (size_t i = 0; i < s_register_count; ++i) {
        const config_manager_register_descriptor_t *desc = &s_register_descriptors[i];
        uint16_t raw_value = s_register_raw_values[i];
        bool is_enum = (desc->value_class == CONFIG_MANAGER_VALUE_ENUM);
        float user_value = is_enum ? (float)raw_value : config_manager_raw_to_user(desc, raw_value);
        float min_user = (desc->has_min && !is_enum) ? config_manager_raw_to_user(desc, desc->min_raw) : 0.0f;
        float max_user = (desc->has_max && !is_enum) ? config_manager_raw_to_user(desc, desc->max_raw) : 0.0f;
        float step_user = (!is_enum) ? desc->step_raw * desc->scale : 0.0f;
        float default_user = is_enum ? (float)desc->default_raw : config_manager_raw_to_user(desc, desc->default_raw);
        const char *access_str = "ro";
        if (desc->access == CONFIG_MANAGER_ACCESS_RW) {
            access_str = "rw";
        } else if (desc->access == CONFIG_MANAGER_ACCESS_WO) {
            access_str = "wo";
        }

        if (!config_manager_json_append(buffer,
                                        buffer_size,
                                        &offset,
                                        "%s{\"key\":\"%s\",\"label\":\"%s\",\"unit\":\"%s\",\"group\":\"%s\","\
                                        "\"type\":\"%s\",\"access\":\"%s\",\"address\":%u,\"scale\":%.6f,"\
                                        "\"precision\":%u,\"value\":%.*f,\"raw\":%u,\"default\":%.*f",
                                        (i == 0) ? "" : ",",
                                        desc->key,
                                        desc->label != NULL ? desc->label : "",
                                        desc->unit != NULL ? desc->unit : "",
                                        desc->group != NULL ? desc->group : "",
                                        desc->type != NULL ? desc->type : "",
                                        access_str,
                                        (unsigned)desc->address,
                                        desc->scale,
                                        (unsigned)desc->precision,
                                        is_enum ? 0 : desc->precision,
                                        user_value,
                                        (unsigned)raw_value,
                                        is_enum ? 0 : desc->precision,
                                        default_user)) {
            result = ESP_ERR_INVALID_SIZE;
            goto cleanup;
        }

        if (!is_enum) {
            if (desc->has_min &&
                !config_manager_json_append(buffer,
                                            buffer_size,
                                            &offset,
                                            ",\"min\":%.*f",
                                            desc->precision,
                                            min_user)) {
                result = ESP_ERR_INVALID_SIZE;
                goto cleanup;
            }
            if (desc->has_max &&
                !config_manager_json_append(buffer,
                                            buffer_size,
                                            &offset,
                                            ",\"max\":%.*f",
                                            desc->precision,
                                            max_user)) {
                result = ESP_ERR_INVALID_SIZE;
                goto cleanup;
            }
            if (desc->step_raw > 0.0f &&
                !config_manager_json_append(buffer,
                                            buffer_size,
                                            &offset,
                                            ",\"step\":%.*f",
                                            desc->precision,
                                            step_user)) {
                result = ESP_ERR_INVALID_SIZE;
                goto cleanup;
            }
        }

        if (desc->comment != NULL &&
            !config_manager_json_append(buffer,
                                        buffer_size,
                                        &offset,
                                        ",\"comment\":\"%s\"",
                                        desc->comment)) {
            result = ESP_ERR_INVALID_SIZE;
            goto cleanup;
        }

        if (desc->enum_count > 0U) {
            if (!config_manager_json_append(buffer,
                                            buffer_size,
                                            &offset,
                                            ",\"enum\":[")) {
                result = ESP_ERR_INVALID_SIZE;
                goto cleanup;
            }
            for (size_t e = 0; e < desc->enum_count; ++e) {
                const config_manager_enum_entry_t *entry = &desc->enum_values[e];
                if (!config_manager_json_append(buffer,
                                                buffer_size,
                                                &offset,
                                                "%s{\"value\":%u,\"label\":\"%s\"}",
                                                (e == 0) ? "" : ",",
                                                (unsigned)entry->value,
                                                entry->label != NULL ? entry->label : "")) {
                    result = ESP_ERR_INVALID_SIZE;
                    goto cleanup;
                }
            }
            if (!config_manager_json_append(buffer, buffer_size, &offset, "]")) {
                result = ESP_ERR_INVALID_SIZE;
                goto cleanup;
            }
        }

        if (!config_manager_json_append(buffer, buffer_size, &offset, "}")) {
            result = ESP_ERR_INVALID_SIZE;
            goto cleanup;
        }
    }

    if (!config_manager_json_append(buffer, buffer_size, &offset, "]}")) {
        result = ESP_ERR_INVALID_SIZE;
        goto cleanup;
    }

    if (out_length != NULL) {
        *out_length = offset;
    }

cleanup:
    config_manager_unlock();
    if (result != ESP_OK && out_length != NULL) {
        *out_length = 0;
    }
    return result;
}

esp_err_t config_manager_apply_register_update_json(const char *json, size_t length)
{
    if (json == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    if (!s_registers_initialised) {
        ESP_LOGW(TAG, "Registers not initialized");
        return ESP_ERR_INVALID_STATE;
    }

    if (length == 0) {
        length = strlen(json);
    }

    if (length >= 2048) {  // CONFIG_MANAGER_MAX_CONFIG_SIZE
        return ESP_ERR_INVALID_SIZE;
    }

    cJSON *root = cJSON_ParseWithLength(json, length);
    if (root == NULL) {
        const char *error = cJSON_GetErrorPtr();
        if (error != NULL) {
            ESP_LOGW(TAG, "Failed to parse register update near: %.32s", error);
        } else {
            ESP_LOGW(TAG, "Failed to parse register update JSON");
        }
        return ESP_ERR_INVALID_ARG;
    }
    if (!cJSON_IsObject(root)) {
        ESP_LOGW(TAG, "Register update payload is not a JSON object");
        cJSON_Delete(root);
        return ESP_ERR_INVALID_ARG;
    }

    const cJSON *key_node = cJSON_GetObjectItemCaseSensitive(root, "key");
    const cJSON *value_node = cJSON_GetObjectItemCaseSensitive(root, "value");
    if (!cJSON_IsString(key_node) || key_node->valuestring == NULL || !cJSON_IsNumber(value_node)) {
        cJSON_Delete(root);
        return ESP_ERR_INVALID_ARG;
    }

    char key[CONFIG_MANAGER_MAX_REGISTER_KEY];
    strncpy(key, key_node->valuestring, sizeof(key) - 1);
    key[sizeof(key) - 1] = '\0';
    float requested_value = (float)value_node->valuedouble;

    cJSON_Delete(root);

    size_t index = 0;
    if (!config_manager_find_register(key, &index)) {
        ESP_LOGW(TAG, "Unknown register key %s", key);
        return ESP_ERR_NOT_FOUND;
    }

    const config_manager_register_descriptor_t *desc = &s_register_descriptors[index];
    uint16_t raw_value = 0;
    esp_err_t conversion = config_manager_convert_user_to_raw(desc, requested_value, &raw_value, NULL);
    if (conversion != ESP_OK) {
        return conversion;
    }

    uint16_t readback_raw = raw_value;
    esp_err_t write_err = uart_bms_write_register(desc->address,
                                                  raw_value,
                                                  &readback_raw,
                                                  UART_BMS_RESPONSE_TIMEOUT_MS);
    if (write_err != ESP_OK) {
        ESP_LOGW(TAG,
                 "Failed to write register %s (0x%04X): %s",
                 desc->key,
                 (unsigned)desc->address,
                 esp_err_to_name(write_err));
        return write_err;
    }

    esp_err_t lock_err = config_manager_lock(CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    s_register_raw_values[index] = readback_raw;

#ifdef ESP_PLATFORM
    esp_err_t persist_err = config_manager_store_register_raw(desc->address, readback_raw);
#endif

    config_manager_unlock();

    config_manager_publish_register_change(desc, readback_raw);

#ifdef ESP_PLATFORM
    if (persist_err != ESP_OK) {
        ESP_LOGW(TAG,
                 "Failed to persist register 0x%04X: %s",
                 (unsigned)desc->address,
                 esp_err_to_name(persist_err));
    }
#endif

    return ESP_OK;
}

size_t config_manager_get_register_count(void)
{
    return s_register_count;
}

bool config_manager_registers_initialized(void)
{
    return s_registers_initialised;
}

void config_manager_registers_reset(void)
{
    s_registers_initialised = false;
    memset(s_register_raw_values, 0, sizeof(s_register_raw_values));
    memset(s_register_events, 0, sizeof(s_register_events));
    s_next_register_event = 0;
}
