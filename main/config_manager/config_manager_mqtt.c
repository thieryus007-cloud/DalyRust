#include "config_manager_mqtt.h"

#include <ctype.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>

#include "esp_log.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"

#include "app_config.h"
#include "mqtt_topics.h"

// External dependencies from config_manager.c
extern const char *TAG;
extern mqtt_client_config_t s_mqtt_config;
extern config_manager_mqtt_topics_t s_mqtt_topics;
extern bool s_mqtt_topics_loaded;
extern mqtt_client_config_t s_mqtt_config_snapshot;
extern config_manager_mqtt_topics_t s_mqtt_topics_snapshot;
extern config_manager_device_settings_t s_device_settings;

// External function dependencies from config_manager.c
extern const char *config_manager_effective_device_name(void);
extern void config_manager_ensure_initialised(void);
extern esp_err_t config_manager_lock(TickType_t timeout);
extern void config_manager_unlock(void);
extern esp_err_t config_manager_store_mqtt_config_to_nvs(const mqtt_client_config_t *config);
extern esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics);
extern esp_err_t config_manager_build_config_snapshot_locked(void);
extern void config_manager_publish_config_snapshot(void);

// Constants from config_manager.c
#ifndef CONFIG_TINYBMS_MQTT_BROKER_URI
#define CONFIG_TINYBMS_MQTT_BROKER_URI "mqtt://localhost"
#endif

#ifndef CONFIG_TINYBMS_MQTT_USERNAME
#define CONFIG_TINYBMS_MQTT_USERNAME ""
#endif

#ifndef CONFIG_TINYBMS_MQTT_PASSWORD
#define CONFIG_TINYBMS_MQTT_PASSWORD ""
#endif

#ifndef CONFIG_TINYBMS_MQTT_KEEPALIVE
#define CONFIG_TINYBMS_MQTT_KEEPALIVE 60
#endif

#ifndef CONFIG_TINYBMS_MQTT_DEFAULT_QOS
#define CONFIG_TINYBMS_MQTT_DEFAULT_QOS 1
#endif

#ifndef CONFIG_TINYBMS_MQTT_RETAIN_STATUS
#define CONFIG_TINYBMS_MQTT_RETAIN_STATUS 0
#endif

#define CONFIG_MANAGER_MQTT_DEFAULT_URI       CONFIG_TINYBMS_MQTT_BROKER_URI
#define CONFIG_MANAGER_MQTT_DEFAULT_USERNAME  CONFIG_TINYBMS_MQTT_USERNAME
#define CONFIG_MANAGER_MQTT_DEFAULT_PASSWORD  CONFIG_TINYBMS_MQTT_PASSWORD
#define CONFIG_MANAGER_MQTT_DEFAULT_KEEPALIVE ((uint16_t)CONFIG_TINYBMS_MQTT_KEEPALIVE)
#define CONFIG_MANAGER_MQTT_DEFAULT_QOS       ((uint8_t)CONFIG_TINYBMS_MQTT_DEFAULT_QOS)
#define CONFIG_MANAGER_MQTT_DEFAULT_RETAIN          (CONFIG_TINYBMS_MQTT_RETAIN_STATUS != 0)
#define CONFIG_MANAGER_MQTT_DEFAULT_CLIENT_CERT     ""
#define CONFIG_MANAGER_MQTT_DEFAULT_CA_CERT         ""
#define CONFIG_MANAGER_MQTT_DEFAULT_VERIFY_HOSTNAME true

#define CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS pdMS_TO_TICKS(1000)

void config_manager_copy_string(char *dest, size_t dest_size, const char *src)
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

void config_manager_copy_topics(config_manager_mqtt_topics_t *dest,
                                const config_manager_mqtt_topics_t *src)
{
    if (dest == NULL || src == NULL) {
        return;
    }

    config_manager_copy_string(dest->status, sizeof(dest->status), src->status);
    config_manager_copy_string(dest->metrics, sizeof(dest->metrics), src->metrics);
    config_manager_copy_string(dest->config, sizeof(dest->config), src->config);
    config_manager_copy_string(dest->can_raw, sizeof(dest->can_raw), src->can_raw);
    config_manager_copy_string(dest->can_decoded, sizeof(dest->can_decoded), src->can_decoded);
    config_manager_copy_string(dest->can_ready, sizeof(dest->can_ready), src->can_ready);
}

void config_manager_make_default_topics_for_name(const char *device_name,
                                                  config_manager_mqtt_topics_t *topics)
{
    if (topics == NULL) {
        return;
    }

    const char *name = (device_name != NULL && device_name[0] != '\0') ? device_name : APP_DEVICE_NAME;

    (void)snprintf(topics->status, sizeof(topics->status), MQTT_TOPIC_FMT_STATUS, name);
    (void)snprintf(topics->metrics, sizeof(topics->metrics), MQTT_TOPIC_FMT_METRICS, name);
    (void)snprintf(topics->config, sizeof(topics->config), MQTT_TOPIC_FMT_CONFIG, name);
    (void)snprintf(topics->can_raw, sizeof(topics->can_raw), MQTT_TOPIC_FMT_CAN_STREAM, name, "raw");
    (void)snprintf(topics->can_decoded, sizeof(topics->can_decoded), MQTT_TOPIC_FMT_CAN_STREAM, name, "decoded");
    (void)snprintf(topics->can_ready, sizeof(topics->can_ready), MQTT_TOPIC_FMT_CAN_STREAM, name, "ready");
}

void config_manager_update_topics_for_device_change(const char *old_name, const char *new_name)
{
    if (old_name == NULL || new_name == NULL || strcmp(old_name, new_name) == 0) {
        return;
    }

    config_manager_mqtt_topics_t old_defaults = {0};
    config_manager_mqtt_topics_t new_defaults = {0};
    config_manager_make_default_topics_for_name(old_name, &old_defaults);
    config_manager_make_default_topics_for_name(new_name, &new_defaults);

    bool updated = false;
    if (strcmp(s_mqtt_topics.status, old_defaults.status) == 0) {
        config_manager_copy_string(s_mqtt_topics.status, sizeof(s_mqtt_topics.status), new_defaults.status);
        updated = true;
    }
    if (strcmp(s_mqtt_topics.metrics, old_defaults.metrics) == 0) {
        config_manager_copy_string(s_mqtt_topics.metrics, sizeof(s_mqtt_topics.metrics), new_defaults.metrics);
        updated = true;
    }
    if (strcmp(s_mqtt_topics.config, old_defaults.config) == 0) {
        config_manager_copy_string(s_mqtt_topics.config, sizeof(s_mqtt_topics.config), new_defaults.config);
        updated = true;
    }
    if (strcmp(s_mqtt_topics.can_raw, old_defaults.can_raw) == 0) {
        config_manager_copy_string(s_mqtt_topics.can_raw, sizeof(s_mqtt_topics.can_raw), new_defaults.can_raw);
        updated = true;
    }
    if (strcmp(s_mqtt_topics.can_decoded, old_defaults.can_decoded) == 0) {
        config_manager_copy_string(s_mqtt_topics.can_decoded, sizeof(s_mqtt_topics.can_decoded), new_defaults.can_decoded);
        updated = true;
    }
    if (strcmp(s_mqtt_topics.can_ready, old_defaults.can_ready) == 0) {
        config_manager_copy_string(s_mqtt_topics.can_ready, sizeof(s_mqtt_topics.can_ready), new_defaults.can_ready);
        updated = true;
    }

    if (updated) {
        config_manager_sanitise_mqtt_topics(&s_mqtt_topics);
        esp_err_t err = config_manager_store_mqtt_topics_to_nvs(&s_mqtt_topics);
        if (err != ESP_OK) {
            ESP_LOGW(TAG, "Failed to persist MQTT topics after device rename: %s", esp_err_to_name(err));
        }
    }
}

void config_manager_reset_mqtt_topics(void)
{
    config_manager_make_default_topics_for_name(config_manager_effective_device_name(), &s_mqtt_topics);
}

void config_manager_sanitise_mqtt_topics(config_manager_mqtt_topics_t *topics)
{
    if (topics == NULL) {
        return;
    }

    config_manager_copy_string(topics->status, sizeof(topics->status), topics->status);
    config_manager_copy_string(topics->metrics, sizeof(topics->metrics), topics->metrics);
    config_manager_copy_string(topics->config, sizeof(topics->config), topics->config);
    config_manager_copy_string(topics->can_raw, sizeof(topics->can_raw), topics->can_raw);
    config_manager_copy_string(topics->can_decoded, sizeof(topics->can_decoded), topics->can_decoded);
    config_manager_copy_string(topics->can_ready, sizeof(topics->can_ready), topics->can_ready);
}


void config_manager_ensure_topics_loaded(void)
{
    if (!s_mqtt_topics_loaded) {
        config_manager_reset_mqtt_topics();
        s_mqtt_topics_loaded = true;
    }
}

void config_manager_lowercase(char *value)
{
    if (value == NULL) {
        return;
    }

    for (size_t i = 0; value[i] != '\0'; ++i) {
        value[i] = (char)tolower((unsigned char)value[i]);
    }
}

uint16_t config_manager_default_port_for_scheme(const char *scheme)
{
    if (scheme != NULL && strcmp(scheme, "mqtts") == 0) {
        return 8883U;
    }
    return 1883U;
}

void config_manager_parse_mqtt_uri(const char *uri,
                                    char *out_scheme,
                                    size_t scheme_size,
                                    char *out_host,
                                    size_t host_size,
                                    uint16_t *out_port)
{
    if (out_scheme != NULL && scheme_size > 0) {
        out_scheme[0] = '\0';
    }
    if (out_host != NULL && host_size > 0) {
        out_host[0] = '\0';
    }
    if (out_port != NULL) {
        *out_port = 1883U;
    }

    char scheme_buffer[16] = "mqtt";
    const char *authority = uri;
    if (uri != NULL) {
        const char *sep = strstr(uri, "://");
        if (sep != NULL) {
            size_t len = (size_t)(sep - uri);
            if (len >= sizeof(scheme_buffer)) {
                len = sizeof(scheme_buffer) - 1U;
            }
            memcpy(scheme_buffer, uri, len);
            scheme_buffer[len] = '\0';
            authority = sep + 3;
        }
    }

    config_manager_lowercase(scheme_buffer);
    if (out_scheme != NULL && scheme_size > 0) {
        config_manager_copy_string(out_scheme, scheme_size, scheme_buffer);
    }

    uint16_t port = config_manager_default_port_for_scheme(scheme_buffer);
    if (authority == NULL) {
        if (out_port != NULL) {
            *out_port = port;
        }
        return;
    }

    const char *path = strpbrk(authority, "/?");
    size_t length = (path != NULL) ? (size_t)(path - authority) : strlen(authority);
    if (length == 0) {
        if (out_port != NULL) {
            *out_port = port;
        }
        return;
    }

    char host_buffer[MQTT_CLIENT_MAX_URI_LENGTH];
    if (length >= sizeof(host_buffer)) {
        length = sizeof(host_buffer) - 1U;
    }
    memcpy(host_buffer, authority, length);
    host_buffer[length] = '\0';

    char *colon = strrchr(host_buffer, ':');
    if (colon != NULL) {
        *colon = '\0';
        ++colon;
        char *endptr = NULL;
        unsigned long parsed = strtoul(colon, &endptr, 10);
        if (endptr != colon && parsed <= UINT16_MAX) {
            port = (uint16_t)parsed;
        }
    }

    if (out_host != NULL && host_size > 0) {
        config_manager_copy_string(out_host, host_size, host_buffer);
    }
    if (out_port != NULL) {
        *out_port = port;
    }
}

void config_manager_sanitise_mqtt_config(mqtt_client_config_t *config)
{
    if (config == NULL) {
        return;
    }

    if (config->keepalive_seconds == 0) {
        config->keepalive_seconds = CONFIG_MANAGER_MQTT_DEFAULT_KEEPALIVE;
    }

    if (config->default_qos > 2U) {
        config->default_qos = 2U;
    }

    if (config->broker_uri[0] == '\0') {
        config_manager_copy_string(config->broker_uri,
                                   sizeof(config->broker_uri),
                                   CONFIG_MANAGER_MQTT_DEFAULT_URI);
    }

    if (config->verify_hostname != true && config->verify_hostname != false) {
        config->verify_hostname = CONFIG_MANAGER_MQTT_DEFAULT_VERIFY_HOSTNAME;
    }
}

const mqtt_client_config_t *config_manager_get_mqtt_client_config(void)
{
    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning MQTT client config without lock");
        return &s_mqtt_config;
    }

    s_mqtt_config_snapshot = s_mqtt_config;
    const mqtt_client_config_t *config = &s_mqtt_config_snapshot;
    config_manager_unlock();
    return config;
}

esp_err_t config_manager_set_mqtt_client_config(const mqtt_client_config_t *config)
{
    if (config == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    mqtt_client_config_t updated = s_mqtt_config;
    config_manager_copy_string(updated.broker_uri, sizeof(updated.broker_uri), config->broker_uri);
    config_manager_copy_string(updated.username, sizeof(updated.username), config->username);
    config_manager_copy_string(updated.password, sizeof(updated.password), config->password);
    config_manager_copy_string(updated.client_cert_path,
                               sizeof(updated.client_cert_path),
                               config->client_cert_path);
    config_manager_copy_string(updated.ca_cert_path,
                               sizeof(updated.ca_cert_path),
                               config->ca_cert_path);
    updated.keepalive_seconds = (config->keepalive_seconds == 0U)
                                    ? CONFIG_MANAGER_MQTT_DEFAULT_KEEPALIVE
                                    : config->keepalive_seconds;
    updated.default_qos = config->default_qos;
    updated.retain_enabled = config->retain_enabled;
    updated.verify_hostname = config->verify_hostname;

    config_manager_sanitise_mqtt_config(&updated);

    esp_err_t err = config_manager_store_mqtt_config_to_nvs(&updated);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to persist MQTT configuration: %s", esp_err_to_name(err));
        config_manager_unlock();
        return err;
    }

    s_mqtt_config = updated;

    esp_err_t snapshot_err = config_manager_build_config_snapshot_locked();
    if (snapshot_err == ESP_OK) {
        config_manager_publish_config_snapshot();
    } else {
        ESP_LOGW(TAG, "Failed to rebuild configuration snapshot: %s", esp_err_to_name(snapshot_err));
    }
    config_manager_unlock();
    return snapshot_err;
}

const config_manager_mqtt_topics_t *config_manager_get_mqtt_topics(void)
{
    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(portMAX_DELAY);
    if (lock_err != ESP_OK) {
        ESP_LOGW(TAG, "Returning MQTT topics without lock");
        return &s_mqtt_topics;
    }

    s_mqtt_topics_snapshot = s_mqtt_topics;
    const config_manager_mqtt_topics_t *topics = &s_mqtt_topics_snapshot;
    config_manager_unlock();
    return topics;
}

esp_err_t config_manager_set_mqtt_topics(const config_manager_mqtt_topics_t *topics)
{
    if (topics == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    config_manager_ensure_initialised();

    esp_err_t lock_err = config_manager_lock(CONFIG_MANAGER_MUTEX_TIMEOUT_TICKS);
    if (lock_err != ESP_OK) {
        return lock_err;
    }

    config_manager_mqtt_topics_t updated = s_mqtt_topics;
    config_manager_copy_topics(&updated, topics);
    config_manager_sanitise_mqtt_topics(&updated);

    esp_err_t err = config_manager_store_mqtt_topics_to_nvs(&updated);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to persist MQTT topics: %s", esp_err_to_name(err));
        config_manager_unlock();
        return err;
    }

    s_mqtt_topics = updated;

    esp_err_t snapshot_err = config_manager_build_config_snapshot_locked();
    if (snapshot_err == ESP_OK) {
        config_manager_publish_config_snapshot();
    } else {
        ESP_LOGW(TAG, "Failed to rebuild configuration snapshot after topic update: %s", esp_err_to_name(snapshot_err));
    }
    config_manager_unlock();
    return snapshot_err;
}
