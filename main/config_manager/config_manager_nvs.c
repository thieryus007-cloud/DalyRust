#include "config_manager_nvs.h"
#include "config_manager_private.h"

#include <string.h>
#include <stdio.h>

#include "esp_log.h"

#ifdef ESP_PLATFORM
#include "nvs_flash.h"
#include "nvs.h"
#include "esp_system.h"
#endif

#include "uart_bms.h"

// ============================================================================
// NVS Key Generation
// ============================================================================

void config_manager_make_register_key(uint16_t address, char *out_key, size_t out_size)
{
    if (out_key == NULL || out_size == 0) {
        return;
    }
    if (snprintf(out_key, out_size, CONFIG_MANAGER_REGISTER_KEY_PREFIX "%04X", (unsigned)address) >= (int)out_size) {
        out_key[out_size - 1] = '\0';
    }
}

// ============================================================================
// WiFi AP Secret Management
// ============================================================================

void config_manager_store_ap_secret_to_nvs(const char *secret)
{
#ifdef ESP_PLATFORM
    if (secret == NULL || secret[0] == '\0') {
        return;
    }

    if (config_manager_init_nvs() != ESP_OK) {
        return;
    }

    nvs_handle_t handle = 0;
    esp_err_t err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to open NVS for AP secret: %s", esp_err_to_name(err));
        return;
    }

    err = nvs_set_str(handle, CONFIG_MANAGER_WIFI_AP_SECRET_KEY, secret);
    if (err == ESP_OK) {
        err = nvs_commit(handle);
    }
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to persist AP secret: %s", esp_err_to_name(err));
    }
    nvs_close(handle);
#else
    (void)secret;
#endif
}

void config_manager_ensure_ap_secret_loaded(void)
{
    if (s_wifi_ap_secret_loaded) {
        return;
    }

#ifdef ESP_PLATFORM
    if (config_manager_init_nvs() != ESP_OK) {
        config_manager_generate_ap_secret(s_wifi_ap_secret, sizeof(s_wifi_ap_secret));
        s_wifi_ap_secret_loaded = true;
        return;
    }

    nvs_handle_t handle = 0;
    esp_err_t err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to open NVS for Wi-Fi secret: %s", esp_err_to_name(err));
        config_manager_generate_ap_secret(s_wifi_ap_secret, sizeof(s_wifi_ap_secret));
        s_wifi_ap_secret_loaded = true;
        return;
    }

    size_t length = sizeof(s_wifi_ap_secret);
    err = nvs_get_str(handle, CONFIG_MANAGER_WIFI_AP_SECRET_KEY, s_wifi_ap_secret, &length);
    if (err == ESP_ERR_NVS_NOT_FOUND ||
        strlen(s_wifi_ap_secret) < CONFIG_MANAGER_WIFI_PASSWORD_MIN_LENGTH) {
        config_manager_generate_ap_secret(s_wifi_ap_secret, sizeof(s_wifi_ap_secret));
        if (strlen(s_wifi_ap_secret) >= CONFIG_MANAGER_WIFI_PASSWORD_MIN_LENGTH) {
            config_manager_store_ap_secret_to_nvs(s_wifi_ap_secret);
        }
    } else if (err != ESP_OK) {
        ESP_LOGW(TAG, "Failed to read AP secret from NVS: %s", esp_err_to_name(err));
        config_manager_generate_ap_secret(s_wifi_ap_secret, sizeof(s_wifi_ap_secret));
    }

    nvs_close(handle);
#else
    config_manager_generate_ap_secret(s_wifi_ap_secret, sizeof(s_wifi_ap_secret));
#endif

    s_wifi_ap_secret_loaded = true;
}

// ============================================================================
// NVS Initialization
// ============================================================================

#ifdef ESP_PLATFORM
esp_err_t config_manager_init_nvs(void)
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

// ============================================================================
// UART Poll Interval Persistence
// ============================================================================

esp_err_t config_manager_store_poll_interval(uint32_t interval_ms)
{
#ifdef ESP_PLATFORM
    esp_err_t err = config_manager_init_nvs();
    if (err != ESP_OK) {
        return err;
    }

    nvs_handle_t handle = 0;
    err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        return err;
    }

    err = nvs_set_u32(handle, CONFIG_MANAGER_POLL_KEY, interval_ms);
    if (err == ESP_OK) {
        err = nvs_commit(handle);
    }
    nvs_close(handle);
    return err;
#else
    (void)interval_ms;
    return ESP_OK;
#endif
}

// ============================================================================
// MQTT Configuration Persistence
// ============================================================================

#ifdef ESP_PLATFORM
esp_err_t config_manager_store_mqtt_config_to_nvs(const mqtt_client_config_t *config)
{
    if (config == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    esp_err_t err = config_manager_init_nvs();
    if (err != ESP_OK) {
        return err;
    }

    nvs_handle_t handle = 0;
    err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        return err;
    }

    // Vérifier tous les set avant commit (transaction atomique)
    bool all_ok = true;

    all_ok &= (nvs_set_str(handle, CONFIG_MANAGER_MQTT_URI_KEY, config->broker_uri) == ESP_OK);
    all_ok &= (nvs_set_str(handle, CONFIG_MANAGER_MQTT_USERNAME_KEY, config->username) == ESP_OK);
    all_ok &= (nvs_set_str(handle, CONFIG_MANAGER_MQTT_PASSWORD_KEY, config->password) == ESP_OK);
    all_ok &= (nvs_set_u16(handle, CONFIG_MANAGER_MQTT_KEEPALIVE_KEY, config->keepalive_seconds) == ESP_OK);
    all_ok &= (nvs_set_u8(handle, CONFIG_MANAGER_MQTT_QOS_KEY, config->default_qos) == ESP_OK);
    all_ok &= (nvs_set_u8(handle, CONFIG_MANAGER_MQTT_RETAIN_KEY, config->retain_enabled ? 1U : 0U) == ESP_OK);
    all_ok &= (nvs_set_str(handle, CONFIG_MANAGER_MQTT_TLS_CLIENT_KEY, config->client_cert_path) == ESP_OK);
    all_ok &= (nvs_set_str(handle, CONFIG_MANAGER_MQTT_TLS_CA_KEY, config->ca_cert_path) == ESP_OK);
    all_ok &= (nvs_set_u8(handle, CONFIG_MANAGER_MQTT_TLS_VERIFY_KEY, config->verify_hostname ? 1U : 0U) == ESP_OK);

    if (!all_ok) {
        ESP_LOGE(TAG, "Failed to set one or more MQTT config values");
        nvs_close(handle);
        return ESP_FAIL;
    }

    err = nvs_commit(handle);
    nvs_close(handle);
    return err;
}

esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics)
{
    if (topics == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    esp_err_t err = config_manager_init_nvs();
    if (err != ESP_OK) {
        return err;
    }

    nvs_handle_t handle = 0;
    err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READWRITE, &handle);
    if (err != ESP_OK) {
        return err;
    }

    err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_STATUS_KEY, topics->status);
    if (err == ESP_OK) {
        err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_MET_KEY, topics->metrics);
    }
    if (err == ESP_OK) {
        err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_CFG_KEY, topics->config);
    }
    if (err == ESP_OK) {
        err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_RAW_KEY, topics->can_raw);
    }
    if (err == ESP_OK) {
        err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_DEC_KEY, topics->can_decoded);
    }
    if (err == ESP_OK) {
        err = nvs_set_str(handle, CONFIG_MANAGER_MQTT_TOPIC_RDY_KEY, topics->can_ready);
    }
    if (err == ESP_OK) {
        err = nvs_commit(handle);
    }

    nvs_close(handle);
    return err;
}
#else
esp_err_t config_manager_store_mqtt_config_to_nvs(const mqtt_client_config_t *config)
{
    (void)config;
    return ESP_OK;
}

esp_err_t config_manager_store_mqtt_topics_to_nvs(const config_manager_mqtt_topics_t *topics)
{
    (void)topics;
    return ESP_OK;
}
#endif

// ============================================================================
// Register Persistence
// ============================================================================

esp_err_t config_manager_store_register_raw(uint16_t address, uint16_t raw_value)
{
#ifdef ESP_PLATFORM
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
#else
    (void)address;
    (void)raw_value;
    return ESP_OK;
#endif
}

bool config_manager_load_register_raw(uint16_t address, uint16_t *out_value)
{
#ifdef ESP_PLATFORM
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
#else
    (void)address;
    (void)out_value;
    return false;
#endif
}

// ============================================================================
// Load All Persistent Settings
// ============================================================================

void config_manager_load_persistent_settings(void)
{
    if (s_settings_loaded) {
        return;
    }

    s_settings_loaded = true;
#ifdef ESP_PLATFORM
    if (config_manager_init_nvs() != ESP_OK) {
        return;
    }

    nvs_handle_t handle = 0;
    esp_err_t err = nvs_open(CONFIG_MANAGER_NAMESPACE, NVS_READONLY, &handle);
    if (err != ESP_OK) {
        return;
    }

    // Load UART poll interval from NVS
    uint32_t stored_interval = 0;
    err = nvs_get_u32(handle, CONFIG_MANAGER_POLL_KEY, &stored_interval);
    if (err == ESP_OK) {
        s_uart_poll_interval_ms = config_manager_clamp_poll_interval(stored_interval);
    }

    // Load MQTT settings from NVS
    config_manager_load_mqtt_settings_from_nvs(handle);
    nvs_close(handle);
#else
    s_uart_poll_interval_ms = UART_BMS_DEFAULT_POLL_INTERVAL_MS;
    config_manager_load_mqtt_settings_from_nvs();
#endif

    // Load configuration file from SPIFFS
    esp_err_t file_err = config_manager_load_config_file(false);
    if (file_err != ESP_OK && file_err != ESP_ERR_NOT_FOUND) {
        ESP_LOGW(TAG, "Failed to load configuration file: %s", esp_err_to_name(file_err));
    }

    // Load register values from NVS
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

    // Apply WiFi AP secret if needed
    config_manager_apply_ap_secret_if_needed(&s_wifi_settings);
}
