/**
 * @file config_manager_json.c
 * @brief JSON serialization and deserialization implementation for configuration manager
 */

#include "config_manager_json.h"

#include <errno.h>
#include <math.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "esp_log.h"

#include "app_config.h"
#include "app_events.h"
#include "mqtt_topics.h"
#include "uart_bms.h"

#ifdef ESP_PLATFORM
#include "esp_spiffs.h"
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#endif

static const char *TAG = "config_json";

#define CONFIG_MANAGER_FS_BASE_PATH "/spiffs"
#define CONFIG_MANAGER_CONFIG_FILE  CONFIG_MANAGER_FS_BASE_PATH "/config.json"

// Static variables for SPIFFS mounting
#ifdef ESP_PLATFORM
static bool s_spiffs_mounted = false;
#endif

// =============================================================================
// Helper Functions
// =============================================================================

/**
 * @brief Copy string safely with size limit
 */
static void config_manager_copy_string(char *dest, size_t dest_size, const char *src)
{
    if (dest == NULL || dest_size == 0) {
        return;
    }

    if (src == NULL) {
        dest[0] = '\0';
        return;
    }

    size_t copy_len = 0;
    while (copy_len + 1 < dest_size && src[copy_len] != '\0') {
        ++copy_len;
    }

    if (copy_len > 0) {
        memcpy(dest, src, copy_len);
    }
    dest[copy_len] = '\0';
}

/**
 * @brief Select secret value or mask based on flag
 */
static const char *config_manager_select_secret_value(const char *value,
                                                       bool include_secrets,
                                                       const char *(*mask_secret)(const char *))
{
    if (value == NULL) {
        return "";
    }
    return include_secrets ? value : mask_secret(value);
}

#ifdef ESP_PLATFORM
/**
 * @brief Mount SPIFFS filesystem
 */
static esp_err_t config_manager_mount_spiffs(void)
{
    if (s_spiffs_mounted) {
        return ESP_OK;
    }

    esp_vfs_spiffs_conf_t conf = {
        .base_path = CONFIG_MANAGER_FS_BASE_PATH,
        .partition_label = NULL,
        .max_files = 4,
        .format_if_mount_failed = true,
    };

    esp_err_t err = esp_vfs_spiffs_register(&conf);
    if (err == ESP_ERR_INVALID_STATE) {
        s_spiffs_mounted = true;
        return ESP_OK;
    }

    if (err == ESP_OK) {
        s_spiffs_mounted = true;
    }
    return err;
}
#endif

// =============================================================================
// JSON Helper Functions (Public API)
// =============================================================================

const cJSON *config_manager_get_object(const cJSON *parent, const char *field)
{
    if (parent == NULL || field == NULL) {
        return NULL;
    }

    const cJSON *candidate = cJSON_GetObjectItemCaseSensitive(parent, field);
    return cJSON_IsObject(candidate) ? candidate : NULL;
}

bool config_manager_copy_json_string(const cJSON *object,
                                     const char *field,
                                     char *dest,
                                     size_t dest_size)
{
    if (object == NULL || field == NULL || dest == NULL || dest_size == 0) {
        return false;
    }

    const cJSON *item = cJSON_GetObjectItemCaseSensitive(object, field);
    if (!cJSON_IsString(item) || item->valuestring == NULL) {
        return false;
    }

    config_manager_copy_string(dest, dest_size, item->valuestring);
    return true;
}

bool config_manager_get_uint32_json(const cJSON *object,
                                   const char *field,
                                   uint32_t *out_value)
{
    if (object == NULL || field == NULL || out_value == NULL) {
        return false;
    }

    const cJSON *item = cJSON_GetObjectItemCaseSensitive(object, field);
    if (!cJSON_IsNumber(item)) {
        return false;
    }

    double value = item->valuedouble;
    if (value < 0.0) {
        value = 0.0;
    }
    if (value > (double)UINT32_MAX) {
        value = (double)UINT32_MAX;
    }

    *out_value = (uint32_t)value;
    return true;
}

bool config_manager_get_int32_json(const cJSON *object,
                                  const char *field,
                                  int32_t *out_value)
{
    if (object == NULL || field == NULL || out_value == NULL) {
        return false;
    }

    const cJSON *item = cJSON_GetObjectItemCaseSensitive(object, field);
    if (!cJSON_IsNumber(item)) {
        return false;
    }

    double value = item->valuedouble;
    if (value < (double)INT32_MIN) {
        value = (double)INT32_MIN;
    }
    if (value > (double)INT32_MAX) {
        value = (double)INT32_MAX;
    }

    *out_value = (int32_t)value;
    return true;
}

// =============================================================================
// Configuration Snapshot Functions
// =============================================================================

/**
 * @brief Render configuration snapshot to JSON buffer
 *
 * @param include_secrets If true, include actual secrets; if false, mask them
 * @param buffer Output buffer for JSON string
 * @param buffer_size Size of output buffer
 * @param out_length Pointer to store actual length of JSON string
 * @param ctx JSON context with configuration state
 * @return ESP_OK on success, error code otherwise
 */
static esp_err_t config_manager_render_config_snapshot(bool include_secrets,
                                                        char *buffer,
                                                        size_t buffer_size,
                                                        size_t *out_length,
                                                        config_manager_json_context_t *ctx)
{
    if (out_length != NULL) {
        *out_length = 0;
    }

    if (buffer == NULL || buffer_size == 0 || ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    // Parse MQTT URI to extract scheme, host, and port
    char scheme[16];
    char host[MQTT_CLIENT_MAX_URI_LENGTH];
    uint16_t port = 0U;
    if (ctx->parse_mqtt_uri != NULL) {
        ctx->parse_mqtt_uri(ctx->mqtt_config->broker_uri, scheme, sizeof(scheme),
                           host, sizeof(host), &port);
    } else {
        scheme[0] = '\0';
        host[0] = '\0';
        port = 1883;
    }

    // Get device name
    const char *device_name = (ctx->effective_device_name != NULL)
                                 ? ctx->effective_device_name()
                                 : APP_DEVICE_NAME;

    // Build version string
    char version[16];
    (void)snprintf(version,
                   sizeof(version),
                   "%u.%u.%u",
                   APP_VERSION_MAJOR,
                   APP_VERSION_MINOR,
                   APP_VERSION_PATCH);

    esp_err_t result = ESP_OK;

#define CHECK_JSON(expr)               \
    do {                               \
        if ((expr) == NULL) {          \
            result = ESP_ERR_NO_MEM;   \
            goto cleanup;              \
        }                              \
    } while (0)

    cJSON *root = cJSON_CreateObject();
    if (root == NULL) {
        return ESP_ERR_NO_MEM;
    }

    // Add top-level configuration fields
    CHECK_JSON(cJSON_AddNumberToObject(root, "uart_poll_interval_ms",
                                       (double)ctx->uart_poll_interval_ms));
    CHECK_JSON(cJSON_AddNumberToObject(root, "uart_poll_interval_min_ms",
                                       (double)UART_BMS_MIN_POLL_INTERVAL_MS));
    CHECK_JSON(cJSON_AddNumberToObject(root, "uart_poll_interval_max_ms",
                                       (double)UART_BMS_MAX_POLL_INTERVAL_MS));

    // Device settings
    cJSON *device = cJSON_CreateObject();
    if (device == NULL) {
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(device, "name", device_name) == NULL ||
        cJSON_AddStringToObject(device, "version", version) == NULL) {
        cJSON_Delete(device);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(root, "device", device);

    // UART settings
    cJSON *uart = cJSON_CreateObject();
    if (uart == NULL) {
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddNumberToObject(uart, "tx_gpio", ctx->uart_pins->tx_gpio) == NULL ||
        cJSON_AddNumberToObject(uart, "rx_gpio", ctx->uart_pins->rx_gpio) == NULL ||
        cJSON_AddNumberToObject(uart, "poll_interval_ms", (double)ctx->uart_poll_interval_ms) == NULL ||
        cJSON_AddNumberToObject(uart, "poll_interval_min_ms", (double)UART_BMS_MIN_POLL_INTERVAL_MS) == NULL ||
        cJSON_AddNumberToObject(uart, "poll_interval_max_ms", (double)UART_BMS_MAX_POLL_INTERVAL_MS) == NULL) {
        cJSON_Delete(uart);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(root, "uart", uart);

    // WiFi settings
    cJSON *wifi = cJSON_CreateObject();
    if (wifi == NULL) {
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }

    // WiFi STA settings
    cJSON *wifi_sta = cJSON_CreateObject();
    if (wifi_sta == NULL) {
        cJSON_Delete(wifi);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(wifi_sta, "ssid", ctx->wifi_settings->sta.ssid) == NULL ||
        cJSON_AddStringToObject(wifi_sta,
                                 "password",
                                 config_manager_select_secret_value(ctx->wifi_settings->sta.password,
                                                                   include_secrets,
                                                                   ctx->mask_secret)) == NULL ||
        cJSON_AddStringToObject(wifi_sta, "hostname", ctx->wifi_settings->sta.hostname) == NULL ||
        cJSON_AddNumberToObject(wifi_sta, "max_retry", (double)ctx->wifi_settings->sta.max_retry) == NULL) {
        cJSON_Delete(wifi_sta);
        cJSON_Delete(wifi);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(wifi, "sta", wifi_sta);

    // WiFi AP settings
    cJSON *wifi_ap = cJSON_CreateObject();
    if (wifi_ap == NULL) {
        cJSON_Delete(wifi);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(wifi_ap, "ssid", ctx->wifi_settings->ap.ssid) == NULL ||
        cJSON_AddStringToObject(wifi_ap,
                                 "password",
                                 config_manager_select_secret_value(ctx->wifi_settings->ap.password,
                                                                   include_secrets,
                                                                   ctx->mask_secret)) == NULL ||
        cJSON_AddNumberToObject(wifi_ap, "channel", (double)ctx->wifi_settings->ap.channel) == NULL ||
        cJSON_AddNumberToObject(wifi_ap, "max_clients", (double)ctx->wifi_settings->ap.max_clients) == NULL) {
        cJSON_Delete(wifi_ap);
        cJSON_Delete(wifi);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(wifi, "ap", wifi_ap);
    cJSON_AddItemToObject(root, "wifi", wifi);

    // CAN settings
    cJSON *can = cJSON_CreateObject();
    if (can == NULL) {
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }

    // CAN TWAI settings
    cJSON *twai = cJSON_CreateObject();
    if (twai == NULL) {
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddNumberToObject(twai, "tx_gpio", ctx->can_settings->twai.tx_gpio) == NULL ||
        cJSON_AddNumberToObject(twai, "rx_gpio", ctx->can_settings->twai.rx_gpio) == NULL) {
        cJSON_Delete(twai);
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(can, "twai", twai);

    // CAN keepalive settings
    cJSON *keepalive = cJSON_CreateObject();
    if (keepalive == NULL) {
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddNumberToObject(keepalive, "interval_ms", (double)ctx->can_settings->keepalive.interval_ms) == NULL ||
        cJSON_AddNumberToObject(keepalive, "timeout_ms", (double)ctx->can_settings->keepalive.timeout_ms) == NULL ||
        cJSON_AddNumberToObject(keepalive, "retry_ms", (double)ctx->can_settings->keepalive.retry_ms) == NULL) {
        cJSON_Delete(keepalive);
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(can, "keepalive", keepalive);

    // CAN publisher settings
    cJSON *publisher = cJSON_CreateObject();
    if (publisher == NULL) {
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddNumberToObject(publisher, "period_ms", (double)ctx->can_settings->publisher.period_ms) == NULL) {
        cJSON_Delete(publisher);
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(can, "publisher", publisher);

    // CAN identity settings
    cJSON *identity = cJSON_CreateObject();
    if (identity == NULL) {
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(identity, "handshake_ascii", ctx->can_settings->identity.handshake_ascii) == NULL ||
        cJSON_AddStringToObject(identity, "manufacturer", ctx->can_settings->identity.manufacturer) == NULL ||
        cJSON_AddStringToObject(identity, "battery_name", ctx->can_settings->identity.battery_name) == NULL ||
        cJSON_AddStringToObject(identity, "battery_family", ctx->can_settings->identity.battery_family) == NULL ||
        cJSON_AddStringToObject(identity, "serial_number", ctx->can_settings->identity.serial_number) == NULL) {
        cJSON_Delete(identity);
        cJSON_Delete(can);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(can, "identity", identity);
    cJSON_AddItemToObject(root, "can", can);

    // MQTT settings
    cJSON *mqtt = cJSON_CreateObject();
    if (mqtt == NULL) {
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(mqtt, "scheme", scheme) == NULL ||
        cJSON_AddStringToObject(mqtt, "broker_uri", ctx->mqtt_config->broker_uri) == NULL ||
        cJSON_AddStringToObject(mqtt, "host", host) == NULL ||
        cJSON_AddNumberToObject(mqtt, "port", (double)port) == NULL ||
        cJSON_AddStringToObject(mqtt, "username", ctx->mqtt_config->username) == NULL ||
        cJSON_AddStringToObject(mqtt,
                                 "password",
                                 config_manager_select_secret_value(ctx->mqtt_config->password,
                                                                   include_secrets,
                                                                   ctx->mask_secret)) == NULL ||
        cJSON_AddStringToObject(mqtt, "client_cert_path", ctx->mqtt_config->client_cert_path) == NULL ||
        cJSON_AddStringToObject(mqtt, "ca_cert_path", ctx->mqtt_config->ca_cert_path) == NULL ||
        cJSON_AddBoolToObject(mqtt, "verify_hostname", ctx->mqtt_config->verify_hostname) == NULL ||
        cJSON_AddNumberToObject(mqtt, "keepalive", (double)ctx->mqtt_config->keepalive_seconds) == NULL ||
        cJSON_AddNumberToObject(mqtt, "default_qos", (double)ctx->mqtt_config->default_qos) == NULL ||
        cJSON_AddBoolToObject(mqtt, "retain", ctx->mqtt_config->retain_enabled) == NULL) {
        cJSON_Delete(mqtt);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }

    // MQTT topics
    cJSON *topics = cJSON_CreateObject();
    if (topics == NULL) {
        cJSON_Delete(mqtt);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    if (cJSON_AddStringToObject(topics, "status", ctx->mqtt_topics->status) == NULL ||
        cJSON_AddStringToObject(topics, "metrics", ctx->mqtt_topics->metrics) == NULL ||
        cJSON_AddStringToObject(topics, "config", ctx->mqtt_topics->config) == NULL ||
        cJSON_AddStringToObject(topics, "can_raw", ctx->mqtt_topics->can_raw) == NULL ||
        cJSON_AddStringToObject(topics, "can_decoded", ctx->mqtt_topics->can_decoded) == NULL ||
        cJSON_AddStringToObject(topics, "can_ready", ctx->mqtt_topics->can_ready) == NULL) {
        cJSON_Delete(topics);
        cJSON_Delete(mqtt);
        result = ESP_ERR_NO_MEM;
        goto cleanup;
    }
    cJSON_AddItemToObject(mqtt, "topics", topics);
    cJSON_AddItemToObject(root, "mqtt", mqtt);

    // Convert to string
    buffer[0] = '\0';
    if (!cJSON_PrintPreallocated(root, buffer, buffer_size, false)) {
        char *json = cJSON_PrintUnformatted(root);
        if (json == NULL) {
            result = ESP_ERR_NO_MEM;
            goto cleanup;
        }
        size_t length = strlen(json);
        if (length >= buffer_size) {
            cJSON_free(json);
            result = ESP_ERR_INVALID_SIZE;
            goto cleanup;
        }
        memcpy(buffer, json, length + 1U);
        if (out_length != NULL) {
            *out_length = length;
        }
        cJSON_free(json);
    } else {
        if (out_length != NULL) {
            *out_length = strlen(buffer);
        }
    }

cleanup:
    cJSON_Delete(root);
#undef CHECK_JSON
    return result;
}

esp_err_t config_manager_build_config_snapshot(config_manager_json_context_t *ctx)
{
    if (ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    // Build full snapshot (with secrets)
    esp_err_t err = config_manager_render_config_snapshot(true,
                                                          ctx->config_json_full,
                                                          ctx->config_json_full_size,
                                                          ctx->config_length_full,
                                                          ctx);
    if (err != ESP_OK) {
        return err;
    }

    // Build public snapshot (without secrets)
    err = config_manager_render_config_snapshot(false,
                                                ctx->config_json_public,
                                                ctx->config_json_public_size,
                                                ctx->config_length_public,
                                                ctx);
    if (err != ESP_OK) {
        return err;
    }

    return ESP_OK;
}

void config_manager_publish_config_snapshot(config_manager_json_context_t *ctx)
{
    if (ctx == NULL || ctx->event_publisher == NULL) {
        return;
    }

    if (*ctx->config_length_public == 0) {
        return;
    }

    event_bus_event_t event = {
        .id = APP_EVENT_ID_CONFIG_UPDATED,
        .payload = ctx->config_json_public,
        .payload_size = *ctx->config_length_public + 1,
    };

#ifdef ESP_PLATFORM
    if (!ctx->event_publisher(&event, pdMS_TO_TICKS(50))) {
        ESP_LOGW(TAG, "Failed to publish configuration snapshot");
    }
#else
    (void)ctx->event_publisher(&event, 50);
#endif
}

// =============================================================================
// Configuration Apply Functions
// =============================================================================

// Forward declarations for functions that need to be provided by config_manager.c
// These are callbacks that the JSON module needs to call
extern uint32_t config_manager_clamp_poll_interval(uint32_t interval_ms);
extern void config_manager_update_topics_for_device_change(const char *old_name, const char *new_name);
extern void config_manager_sanitise_mqtt_config(mqtt_client_config_t *config);
extern void config_manager_sanitise_mqtt_topics(config_manager_mqtt_topics_t *topics);
extern void config_manager_apply_ap_secret_if_needed(config_manager_wifi_settings_internal_t *wifi);
extern esp_err_t config_manager_store_poll_interval(uint32_t interval_ms);
extern esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics);

esp_err_t config_manager_apply_config_payload(const char *json,
                                             size_t length,
                                             bool persist,
                                             bool apply_runtime,
                                             config_manager_json_context_t *ctx)
{
    if (json == NULL || ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    if (length == 0U) {
        length = strlen(json);
    }
    if (length >= CONFIG_MANAGER_MAX_CONFIG_SIZE) {
        ESP_LOGW(TAG, "Config payload too large: %u bytes", (unsigned)length);
        return ESP_ERR_INVALID_SIZE;
    }

    cJSON *root = cJSON_ParseWithLength(json, length);
    if (root == NULL) {
        const char *error = cJSON_GetErrorPtr();
        if (error != NULL) {
            ESP_LOGW(TAG, "Failed to parse configuration JSON near: %.32s", error);
        } else {
            ESP_LOGW(TAG, "Failed to parse configuration JSON");
        }
        return ESP_ERR_INVALID_ARG;
    }
    if (!cJSON_IsObject(root)) {
        ESP_LOGW(TAG, "Configuration payload is not a JSON object");
        cJSON_Delete(root);
        return ESP_ERR_INVALID_ARG;
    }

    // Create working copies of all settings
    config_manager_device_settings_internal_t device = *ctx->device_settings;
    config_manager_uart_pins_internal_t uart_pins = *ctx->uart_pins;
    config_manager_wifi_settings_internal_t wifi = *ctx->wifi_settings;
    config_manager_can_settings_internal_t can = *ctx->can_settings;
    uint32_t poll_interval = ctx->uart_poll_interval_ms;
    bool poll_interval_updated = false;
    bool sta_credentials_changed = false;

    // Parse device settings
    const cJSON *device_obj = config_manager_get_object(root, "device");
    if (device_obj != NULL) {
        config_manager_copy_json_string(device_obj, "name", device.name, sizeof(device.name));
    }

    // Parse UART settings
    const cJSON *uart_obj = config_manager_get_object(root, "uart");
    if (uart_obj != NULL) {
        uint32_t poll = 0U;
        if (config_manager_get_uint32_json(uart_obj, "poll_interval_ms", &poll)) {
            poll_interval = config_manager_clamp_poll_interval(poll);
            poll_interval_updated = true;
        }

        int32_t gpio = 0;
        if (config_manager_get_int32_json(uart_obj, "tx_gpio", &gpio)) {
            if (gpio < -1) {
                gpio = -1;
            }
            if (gpio > 48) {
                gpio = 48;
            }
            uart_pins.tx_gpio = (int)gpio;
        }
        if (config_manager_get_int32_json(uart_obj, "rx_gpio", &gpio)) {
            if (gpio < -1) {
                gpio = -1;
            }
            if (gpio > 48) {
                gpio = 48;
            }
            uart_pins.rx_gpio = (int)gpio;
        }
    } else {
        // Check for legacy top-level uart_poll_interval_ms
        uint32_t poll = 0U;
        if (config_manager_get_uint32_json(root, "uart_poll_interval_ms", &poll)) {
            poll_interval = config_manager_clamp_poll_interval(poll);
            poll_interval_updated = true;
        }
    }

    // Parse WiFi settings
    const cJSON *wifi_obj = config_manager_get_object(root, "wifi");
    if (wifi_obj != NULL) {
        const cJSON *sta_obj = config_manager_get_object(wifi_obj, "sta");
        if (sta_obj != NULL) {
            config_manager_copy_json_string(sta_obj, "ssid", wifi.sta.ssid, sizeof(wifi.sta.ssid));
            config_manager_copy_json_string(sta_obj, "password", wifi.sta.password, sizeof(wifi.sta.password));
            config_manager_copy_json_string(sta_obj, "hostname", wifi.sta.hostname, sizeof(wifi.sta.hostname));

            uint32_t max_retry = 0U;
            if (config_manager_get_uint32_json(sta_obj, "max_retry", &max_retry)) {
                if (max_retry > 255U) {
                    max_retry = 255U;
                }
                wifi.sta.max_retry = (uint8_t)max_retry;
            }
        }

        const cJSON *ap_obj = config_manager_get_object(wifi_obj, "ap");
        if (ap_obj != NULL) {
            config_manager_copy_json_string(ap_obj, "ssid", wifi.ap.ssid, sizeof(wifi.ap.ssid));
            config_manager_copy_json_string(ap_obj, "password", wifi.ap.password, sizeof(wifi.ap.password));

            uint32_t channel = 0U;
            if (config_manager_get_uint32_json(ap_obj, "channel", &channel)) {
                if (channel < 1U) {
                    channel = 1U;
                }
                if (channel > 13U) {
                    channel = 13U;
                }
                wifi.ap.channel = (uint8_t)channel;
            }

            uint32_t max_clients = 0U;
            if (config_manager_get_uint32_json(ap_obj, "max_clients", &max_clients)) {
                if (max_clients < 1U) {
                    max_clients = 1U;
                }
                if (max_clients > 10U) {
                    max_clients = 10U;
                }
                wifi.ap.max_clients = (uint8_t)max_clients;
            }
        }
    }

    // Check if STA credentials changed
    sta_credentials_changed = (strcmp(wifi.sta.ssid, ctx->wifi_settings->sta.ssid) != 0) ||
                              (strcmp(wifi.sta.password, ctx->wifi_settings->sta.password) != 0);

    // Apply AP secret if needed
    config_manager_apply_ap_secret_if_needed(&wifi);

    // Parse CAN settings
    const cJSON *can_obj = config_manager_get_object(root, "can");
    if (can_obj != NULL) {
        const cJSON *twai_obj = config_manager_get_object(can_obj, "twai");
        if (twai_obj != NULL) {
            int32_t gpio = 0;
            if (config_manager_get_int32_json(twai_obj, "tx_gpio", &gpio)) {
                if (gpio < -1) {
                    gpio = -1;
                }
                if (gpio > 39) {
                    gpio = 39;
                }
                can.twai.tx_gpio = (int)gpio;
            }
            if (config_manager_get_int32_json(twai_obj, "rx_gpio", &gpio)) {
                if (gpio < -1) {
                    gpio = -1;
                }
                if (gpio > 39) {
                    gpio = 39;
                }
                can.twai.rx_gpio = (int)gpio;
            }
        }

        const cJSON *keepalive_obj = config_manager_get_object(can_obj, "keepalive");
        if (keepalive_obj != NULL) {
            uint32_t value = 0U;
            if (config_manager_get_uint32_json(keepalive_obj, "interval_ms", &value)) {
                if (value < 10U) {
                    value = 10U;
                }
                if (value > 600000U) {
                    value = 600000U;
                }
                can.keepalive.interval_ms = value;
            }
            if (config_manager_get_uint32_json(keepalive_obj, "timeout_ms", &value)) {
                if (value < 100U) {
                    value = 100U;
                }
                if (value > 600000U) {
                    value = 600000U;
                }
                can.keepalive.timeout_ms = value;
            }
            if (config_manager_get_uint32_json(keepalive_obj, "retry_ms", &value)) {
                if (value < 10U) {
                    value = 10U;
                }
                if (value > 600000U) {
                    value = 600000U;
                }
                can.keepalive.retry_ms = value;
            }
        }

        const cJSON *publisher_obj = config_manager_get_object(can_obj, "publisher");
        if (publisher_obj != NULL) {
            uint32_t value = 0U;
            if (config_manager_get_uint32_json(publisher_obj, "period_ms", &value)) {
                if (value > 600000U) {
                    value = 600000U;
                }
                can.publisher.period_ms = value;
            }
        }

        const cJSON *identity_obj = config_manager_get_object(can_obj, "identity");
        if (identity_obj != NULL) {
            config_manager_copy_json_string(identity_obj,
                                            "handshake_ascii",
                                            can.identity.handshake_ascii,
                                            sizeof(can.identity.handshake_ascii));
            config_manager_copy_json_string(identity_obj,
                                            "manufacturer",
                                            can.identity.manufacturer,
                                            sizeof(can.identity.manufacturer));
            config_manager_copy_json_string(identity_obj,
                                            "battery_name",
                                            can.identity.battery_name,
                                            sizeof(can.identity.battery_name));
            config_manager_copy_json_string(identity_obj,
                                            "battery_family",
                                            can.identity.battery_family,
                                            sizeof(can.identity.battery_family));
            config_manager_copy_json_string(identity_obj,
                                            "serial_number",
                                            can.identity.serial_number,
                                            sizeof(can.identity.serial_number));
        }
    }

    cJSON_Delete(root);

    // NOTE: The actual application of these settings needs to be done by config_manager.c
    // because it has access to the internal state and mutex. This function can only parse
    // and validate the JSON. The caller must apply the parsed settings.
    //
    // This is a limitation of the current extraction approach. A better approach would be
    // to have config_manager.c provide callback functions to apply settings, or to refactor
    // the state management to be accessible from this module.

    ESP_LOGW(TAG, "config_manager_apply_config_payload: Parsing complete but application logic needs implementation");
    ESP_LOGW(TAG, "The parsed settings need to be applied by the caller (config_manager.c)");

    // Return an error indicating incomplete implementation
    return ESP_ERR_NOT_SUPPORTED;
}

// =============================================================================
// File I/O Functions
// =============================================================================

esp_err_t config_manager_save_config_file(config_manager_json_context_t *ctx)
{
    if (ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

#ifdef ESP_PLATFORM
    esp_err_t mount_err = config_manager_mount_spiffs();
    if (mount_err != ESP_OK) {
        ESP_LOGW(TAG, "Unable to mount SPIFFS for config save: %s", esp_err_to_name(mount_err));
        return mount_err;
    }
#endif

    FILE *file = fopen(CONFIG_MANAGER_CONFIG_FILE, "w");
    if (file == NULL) {
        ESP_LOGW(TAG, "Failed to open %s for writing: errno=%d", CONFIG_MANAGER_CONFIG_FILE, errno);
        return ESP_FAIL;
    }

    size_t written = fwrite(ctx->config_json_full, 1, *ctx->config_length_full, file);
    int flush_result = fflush(file);
    int close_result = fclose(file);
    if (written != *ctx->config_length_full || flush_result != 0 || close_result != 0) {
        ESP_LOGW(TAG,
                 "Failed to write configuration file (written=%zu expected=%zu errno=%d)",
                 written,
                 *ctx->config_length_full,
                 errno);
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Configuration saved to %s (%zu bytes)", CONFIG_MANAGER_CONFIG_FILE, written);
    return ESP_OK;
}

esp_err_t config_manager_load_config_file(bool apply_runtime,
                                         config_manager_json_context_t *ctx)
{
    if (ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

#ifdef ESP_PLATFORM
    esp_err_t mount_err = config_manager_mount_spiffs();
    if (mount_err != ESP_OK) {
        ESP_LOGW(TAG, "Unable to mount SPIFFS for config load: %s", esp_err_to_name(mount_err));
        return mount_err;
    }
#endif

    FILE *file = fopen(CONFIG_MANAGER_CONFIG_FILE, "r");
    if (file == NULL) {
        return ESP_ERR_NOT_FOUND;
    }

    char buffer[CONFIG_MANAGER_MAX_CONFIG_SIZE];
    size_t read = fread(buffer, 1, sizeof(buffer) - 1U, file);
    fclose(file);
    if (read == 0U) {
        ESP_LOGW(TAG, "Configuration file %s is empty", CONFIG_MANAGER_CONFIG_FILE);
        return ESP_ERR_INVALID_SIZE;
    }

    buffer[read] = '\0';
    esp_err_t err = config_manager_apply_config_payload(buffer, read, false, apply_runtime, ctx);
    if (err == ESP_OK) {
        ESP_LOGI(TAG, "Configuration loaded from %s (%zu bytes)", CONFIG_MANAGER_CONFIG_FILE, read);
    }
    return err;
}

// =============================================================================
// Public API Implementation Functions
// =============================================================================

esp_err_t config_manager_get_config_json_impl(char *buffer,
                                              size_t buffer_size,
                                              size_t *out_length,
                                              config_manager_snapshot_flags_t flags,
                                              config_manager_json_context_t *ctx)
{
    if (buffer == NULL || buffer_size == 0 || ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    bool include_secrets = ((flags & CONFIG_MANAGER_SNAPSHOT_INCLUDE_SECRETS) != 0);
    const char *source = include_secrets ? ctx->config_json_full : ctx->config_json_public;
    size_t length = include_secrets ? *ctx->config_length_full : *ctx->config_length_public;

    if (length + 1 > buffer_size) {
        return ESP_ERR_INVALID_SIZE;
    }

    memcpy(buffer, source, length + 1);
    if (out_length != NULL) {
        *out_length = length;
    }

    return ESP_OK;
}

esp_err_t config_manager_set_config_json_impl(const char *json,
                                              size_t length,
                                              config_manager_json_context_t *ctx)
{
    if (json == NULL || ctx == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    return config_manager_apply_config_payload(json, length, true, true, ctx);
}
