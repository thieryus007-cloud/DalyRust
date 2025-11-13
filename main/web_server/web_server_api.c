#include "web_server_api.h"
#include "web_server_auth.h"
#include "web_server.h"

#include <ctype.h>
#include <inttypes.h>
#include <stdbool.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <strings.h>
#include <limits.h>

#include "esp_err.h"
#include "esp_http_server.h"
#include "esp_log.h"

#include "cJSON.h"

#include "config_manager.h"
#include "monitoring.h"
#include "mqtt_client.h"
#include "mqtt_gateway.h"
#include "system_metrics.h"

// =============================================================================
// Helper functions
// =============================================================================

static void web_server_parse_mqtt_uri(const char *uri,
                                      char *scheme,
                                      size_t scheme_size,
                                      char *host,
                                      size_t host_size,
                                      uint16_t *port_out)
{
    if (scheme != NULL && scheme_size > 0) {
        scheme[0] = '\0';
    }

    if (host != NULL && host_size > 0) {
        host[0] = '\0';
    }
    if (port_out != NULL) {
        *port_out = 1883U;
    }

    if (uri == NULL) {
        if (scheme != NULL && scheme_size > 0) {
            (void)snprintf(scheme, scheme_size, "%s", "mqtt");
        }
        return;
    }

    const char *authority = uri;
    const char *sep = strstr(uri, "://");
    char scheme_buffer[16] = "mqtt";
    if (sep != NULL) {
        size_t len = (size_t)(sep - uri);
        if (len >= sizeof(scheme_buffer)) {
            len = sizeof(scheme_buffer) - 1U;
        }
        memcpy(scheme_buffer, uri, len);
        scheme_buffer[len] = '\0';
        authority = sep + 3;
    }

    for (size_t i = 0; scheme_buffer[i] != '\0'; ++i) {
        scheme_buffer[i] = (char)tolower((unsigned char)scheme_buffer[i]);
    }
    if (scheme != NULL && scheme_size > 0) {
        (void)snprintf(scheme, scheme_size, "%s", scheme_buffer);
    }

    uint16_t port = (strcmp(scheme_buffer, "mqtts") == 0) ? 8883U : 1883U;
    if (authority == NULL || authority[0] == '\0') {
        if (port_out != NULL) {
            *port_out = port;
        }
        return;
    }

    const char *path = strpbrk(authority, "/?");
    size_t length = (path != NULL) ? (size_t)(path - authority) : strlen(authority);
    if (length == 0) {
        if (port_out != NULL) {
            *port_out = port;
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

    if (host != NULL && host_size > 0) {
        (void)snprintf(host, host_size, "%s", host_buffer);
    }
    if (port_out != NULL) {
        *port_out = port;
    }
}

static bool web_server_query_value_truthy(const char *value, size_t length)
{
    if (value == NULL || length == 0U) {
        return true;
    }

    if (length == 1U) {
        char c = (char)tolower((unsigned char)value[0]);
        return (c == '1') || (c == 'y') || (c == 't');
    }

    if (length == 2U && strncasecmp(value, "on", 2) == 0) {
        return true;
    }
    if (length == 3U && strncasecmp(value, "yes", 3) == 0) {
        return true;
    }
    if (length == 4U && strncasecmp(value, "true", 4) == 0) {
        return true;
    }

    return false;
}

static const char *web_server_mqtt_event_to_string(mqtt_client_event_id_t id)
{
    switch (id) {
        case MQTT_CLIENT_EVENT_CONNECTED:
            return "connected";
        case MQTT_CLIENT_EVENT_DISCONNECTED:
            return "disconnected";
        case MQTT_CLIENT_EVENT_SUBSCRIBED:
            return "subscribed";
        case MQTT_CLIENT_EVENT_PUBLISHED:
            return "published";
        case MQTT_CLIENT_EVENT_DATA:
            return "data";
        case MQTT_CLIENT_EVENT_ERROR:
            return "error";
        default:
            return "unknown";
    }
}

// =============================================================================
// Metrics API handlers
// =============================================================================

esp_err_t web_server_api_metrics_runtime_handler(httpd_req_t *req)
{
    system_metrics_runtime_t runtime;
    esp_err_t err = system_metrics_collect_runtime(&runtime);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to collect runtime metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Runtime metrics unavailable");
        return err;
    }

    char *buffer = malloc(WEB_SERVER_RUNTIME_JSON_SIZE);
    if (buffer == NULL) {
        httpd_resp_send_err(req, HTTPD_503_SERVICE_UNAVAILABLE, "Memory allocation failure");
        return ESP_ERR_NO_MEM;
    }

    size_t length = 0;
    err = system_metrics_runtime_to_json(&runtime, buffer, WEB_SERVER_RUNTIME_JSON_SIZE, &length);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to serialize runtime metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Runtime metrics serialization error");
        free(buffer);
        return err;
    }

    esp_err_t send_err = web_server_send_json(req, buffer, length);
    free(buffer);
    return send_err;
}

esp_err_t web_server_api_event_bus_metrics_handler(httpd_req_t *req)
{
    system_metrics_event_bus_metrics_t metrics;
    esp_err_t err = system_metrics_collect_event_bus(&metrics);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to collect event bus metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Event bus metrics unavailable");
        return err;
    }

    char *buffer = malloc(WEB_SERVER_EVENT_BUS_JSON_SIZE);
    if (buffer == NULL) {
        httpd_resp_send_err(req, HTTPD_503_SERVICE_UNAVAILABLE, "Memory allocation failure");
        return ESP_ERR_NO_MEM;
    }

    size_t length = 0;
    err = system_metrics_event_bus_to_json(&metrics, buffer, WEB_SERVER_EVENT_BUS_JSON_SIZE, &length);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to serialize event bus metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Event bus metrics serialization error");
        free(buffer);
        return err;
    }

    esp_err_t send_err = web_server_send_json(req, buffer, length);
    free(buffer);
    return send_err;
}

esp_err_t web_server_api_system_tasks_handler(httpd_req_t *req)
{
    system_metrics_task_snapshot_t tasks;
    esp_err_t err = system_metrics_collect_tasks(&tasks);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to collect task metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Task metrics unavailable");
        return err;
    }

    char *buffer = malloc(WEB_SERVER_TASKS_JSON_SIZE);
    if (buffer == NULL) {
        httpd_resp_send_err(req, HTTPD_503_SERVICE_UNAVAILABLE, "Memory allocation failure");
        return ESP_ERR_NO_MEM;
    }

    size_t length = 0;
    err = system_metrics_tasks_to_json(&tasks, buffer, WEB_SERVER_TASKS_JSON_SIZE, &length);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to serialize task metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Task metrics serialization error");
        free(buffer);
        return err;
    }

    esp_err_t send_err = web_server_send_json(req, buffer, length);
    free(buffer);
    return send_err;
}

esp_err_t web_server_api_system_modules_handler(httpd_req_t *req)
{
    system_metrics_event_bus_metrics_t event_bus_metrics;
    esp_err_t err = system_metrics_collect_event_bus(&event_bus_metrics);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to collect event bus metrics for modules: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Module metrics unavailable");
        return err;
    }

    system_metrics_module_snapshot_t modules;
    err = system_metrics_collect_modules(&modules, &event_bus_metrics);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to aggregate module metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Module metrics unavailable");
        return err;
    }

    char *buffer = malloc(WEB_SERVER_MODULES_JSON_SIZE);
    if (buffer == NULL) {
        httpd_resp_send_err(req, HTTPD_503_SERVICE_UNAVAILABLE, "Memory allocation failure");
        return ESP_ERR_NO_MEM;
    }

    size_t length = 0;
    err = system_metrics_modules_to_json(&modules, buffer, WEB_SERVER_MODULES_JSON_SIZE, &length);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to serialize module metrics: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Module metrics serialization error");
        free(buffer);
        return err;
    }

    esp_err_t send_err = web_server_send_json(req, buffer, length);
    free(buffer);
    return send_err;
}

// =============================================================================
// Status API handler
// =============================================================================

esp_err_t web_server_api_status_handler(httpd_req_t *req)
{
    char snapshot[MONITORING_SNAPSHOT_MAX_SIZE];
    size_t length = 0;
    esp_err_t err = monitoring_get_status_json(snapshot, sizeof(snapshot), &length);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to build status JSON: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Status unavailable");
        return err;
    }

    if (length >= sizeof(snapshot)) {
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Status too large");
        return ESP_ERR_INVALID_SIZE;
    }

    snapshot[length] = '\0';

    char response[MONITORING_SNAPSHOT_MAX_SIZE + 32U];
    int written = snprintf(response, sizeof(response), "{\"battery\":%s}", snapshot);
    if (written <= 0 || (size_t)written >= sizeof(response)) {
        ESP_LOGE(TAG, "Failed to wrap status response");
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Status unavailable");
        return ESP_ERR_INVALID_SIZE;
    }

    httpd_resp_set_type(req, "application/json");
    httpd_resp_set_hdr(req, "Cache-Control", "no-store");
    return httpd_resp_send(req, response, written);
}

// =============================================================================
// Configuration API handlers
// =============================================================================

esp_err_t web_server_api_config_get_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, false, NULL, 0)) {
        return ESP_FAIL;
    }

    char buffer[CONFIG_MANAGER_MAX_CONFIG_SIZE];
    size_t length = 0;
    const char *visibility = NULL;
    bool authorized = web_server_request_authorized_for_secrets(req);
    esp_err_t err = web_server_prepare_config_snapshot(req->uri,
                                                       authorized,
                                                       buffer,
                                                       sizeof(buffer),
                                                       &length,
                                                       &visibility);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to load configuration JSON: %s", esp_err_to_name(err));
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Config unavailable");
        return err;
    }

    httpd_resp_set_type(req, "application/json");
    httpd_resp_set_hdr(req, "Cache-Control", "no-store");
    if (visibility != NULL) {
        httpd_resp_set_hdr(req, "X-Config-Snapshot", visibility);
    }
    return httpd_resp_send(req, buffer, length);
}

esp_err_t web_server_api_config_post_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, true, NULL, 0)) {
        return ESP_FAIL;
    }

    if (req->content_len == 0) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Empty body");
        return ESP_ERR_INVALID_SIZE;
    }

    if (req->content_len + 1 > CONFIG_MANAGER_MAX_CONFIG_SIZE) {
        httpd_resp_send_err(req, HTTPD_413_PAYLOAD_TOO_LARGE, "Config too large");
        return ESP_ERR_INVALID_SIZE;
    }

    char buffer[CONFIG_MANAGER_MAX_CONFIG_SIZE];
    size_t received = 0;
    while (received < req->content_len) {
        int ret = httpd_req_recv(req, buffer + received, req->content_len - received);
        if (ret <= 0) {
            if (ret == HTTPD_SOCK_ERR_TIMEOUT) {
                continue;
            }
            ESP_LOGE(TAG, "Error receiving config payload: %d", ret);
            httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Read error");
            return ESP_FAIL;
        }
        received += ret;
    }

    buffer[received] = '\0';

    esp_err_t err = config_manager_set_config_json(buffer, received);
    if (err != ESP_OK) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Invalid configuration");
        return err;
    }

    httpd_resp_set_type(req, "application/json");
    return httpd_resp_sendstr(req, "{\"status\":\"updated\"}");
}

// =============================================================================
// MQTT Configuration API handlers
// =============================================================================

esp_err_t web_server_api_mqtt_config_get_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, false, NULL, 0)) {
        return ESP_FAIL;
    }

    const mqtt_client_config_t *config = config_manager_get_mqtt_client_config();
    const config_manager_mqtt_topics_t *topics = config_manager_get_mqtt_topics();
    if (config == NULL || topics == NULL) {
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "MQTT config unavailable");
        return ESP_FAIL;
    }

    char scheme[16];
    char host[MQTT_CLIENT_MAX_URI_LENGTH];
    uint16_t port = 0U;
    web_server_parse_mqtt_uri(config->broker_uri, scheme, sizeof(scheme), host, sizeof(host), &port);

    // Mask password for security - never send actual password in GET response
    const char *masked_password = config_manager_mask_secret(config->password);

    char buffer[WEB_SERVER_MQTT_JSON_SIZE];
    int written = snprintf(buffer,
                           sizeof(buffer),
                           "{\"scheme\":\"%s\",\"broker_uri\":\"%s\",\"host\":\"%s\",\"port\":%u,"
                           "\"username\":\"%s\",\"password\":\"%s\",\"client_cert_path\":\"%s\","
                           "\"ca_cert_path\":\"%s\",\"verify_hostname\":%s,\"keepalive\":%u,\"default_qos\":%u,"
                           "\"retain\":%s,\"topics\":{\"status\":\"%s\",\"metrics\":\"%s\",\"config\":\"%s\","
                           "\"can_raw\":\"%s\",\"can_decoded\":\"%s\",\"can_ready\":\"%s\"}}",
                           scheme,
                           config->broker_uri,
                           host,
                           (unsigned)port,
                           config->username,
                           masked_password,
                           config->client_cert_path,
                           config->ca_cert_path,
                           config->verify_hostname ? "true" : "false",
                           (unsigned)config->keepalive_seconds,
                           (unsigned)config->default_qos,
                           config->retain_enabled ? "true" : "false",
                           topics->status,
                           topics->metrics,
                           topics->config,
                           topics->can_raw,
                           topics->can_decoded,
                           topics->can_ready);
    if (written < 0 || written >= (int)sizeof(buffer)) {
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "MQTT config too large");
        return ESP_ERR_INVALID_SIZE;
    }

    httpd_resp_set_type(req, "application/json");
    httpd_resp_set_hdr(req, "Cache-Control", "no-store");
    return httpd_resp_send(req, buffer, written);
}

esp_err_t web_server_api_mqtt_config_post_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, true, NULL, 0)) {
        return ESP_FAIL;
    }

    if (req->content_len == 0) {
        httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Empty body");
        return ESP_ERR_INVALID_SIZE;
    }

    if (req->content_len + 1 >= CONFIG_MANAGER_MAX_CONFIG_SIZE) {
        httpd_resp_send_err(req, HTTPD_413_PAYLOAD_TOO_LARGE, "Payload too large");
        return ESP_ERR_INVALID_SIZE;
    }

    char payload[CONFIG_MANAGER_MAX_CONFIG_SIZE];
    size_t received = 0;
    while (received < (size_t)req->content_len) {
        int ret = httpd_req_recv(req, payload + received, req->content_len - received);
        if (ret <= 0) {
            if (ret == HTTPD_SOCK_ERR_TIMEOUT) {
                continue;
            }
            httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Read error");
            return ESP_FAIL;
        }
        received += (size_t)ret;
    }
    payload[received] = '\0';

    const mqtt_client_config_t *current = config_manager_get_mqtt_client_config();
    const config_manager_mqtt_topics_t *current_topics = config_manager_get_mqtt_topics();
    if (current == NULL || current_topics == NULL) {
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "MQTT config unavailable");
        return ESP_FAIL;
    }

    mqtt_client_config_t updated = *current;
    config_manager_mqtt_topics_t topics = *current_topics;

    char default_scheme[16];
    char default_host[MQTT_CLIENT_MAX_URI_LENGTH];
    uint16_t default_port = 0U;
    web_server_parse_mqtt_uri(updated.broker_uri,
                              default_scheme,
                              sizeof(default_scheme),
                              default_host,
                              sizeof(default_host),
                              &default_port);

    char scheme[sizeof(default_scheme)];
    snprintf(scheme, sizeof(scheme), "%s", default_scheme);
    char host[sizeof(default_host)];
    snprintf(host, sizeof(host), "%s", default_host);
    uint16_t port = default_port;

    esp_err_t status = ESP_OK;
    bool send_error = false;
    int error_status = HTTPD_400_BAD_REQUEST;
    const char *error_message = "Invalid MQTT configuration";

    cJSON *root = cJSON_ParseWithLength(payload, received);
    if (root == NULL || !cJSON_IsObject(root)) {
        status = ESP_ERR_INVALID_ARG;
        send_error = true;
        error_message = "Invalid JSON payload";
        goto cleanup;
    }

    const cJSON *item = NULL;

    item = cJSON_GetObjectItemCaseSensitive(root, "scheme");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "scheme must be a string";
            goto cleanup;
        }
        snprintf(scheme, sizeof(scheme), "%s", item->valuestring);
        for (size_t i = 0; scheme[i] != '\0'; ++i) {
            scheme[i] = (char)tolower((unsigned char)scheme[i]);
        }
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "host");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "host must be a string";
            goto cleanup;
        }
        snprintf(host, sizeof(host), "%s", item->valuestring);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "port");
    if (item != NULL) {
        if (!cJSON_IsNumber(item)) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "port must be a number";
            goto cleanup;
        }
        double value = item->valuedouble;
        if ((double)item->valueint != value || value < 1.0 || value > UINT16_MAX) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "Invalid port";
            goto cleanup;
        }
        port = (uint16_t)item->valueint;
    }

    if (host[0] == '\0') {
        status = ESP_ERR_INVALID_ARG;
        send_error = true;
        error_message = "Host is required";
        goto cleanup;
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "username");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "username must be a string";
            goto cleanup;
        }
        snprintf(updated.username, sizeof(updated.username), "%s", item->valuestring);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "password");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "password must be a string";
            goto cleanup;
        }
        snprintf(updated.password, sizeof(updated.password), "%s", item->valuestring);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "client_cert_path");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "client_cert_path must be a string";
            goto cleanup;
        }
        snprintf(updated.client_cert_path, sizeof(updated.client_cert_path), "%s", item->valuestring);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "ca_cert_path");
    if (item != NULL) {
        if (!cJSON_IsString(item) || item->valuestring == NULL) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "ca_cert_path must be a string";
            goto cleanup;
        }
        snprintf(updated.ca_cert_path, sizeof(updated.ca_cert_path), "%s", item->valuestring);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "verify_hostname");
    if (item != NULL) {
        if (!cJSON_IsBool(item)) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "verify_hostname must be a boolean";
            goto cleanup;
        }
        updated.verify_hostname = cJSON_IsTrue(item);
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "keepalive");
    if (item != NULL) {
        if (!cJSON_IsNumber(item) || item->valuedouble < 0.0) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "keepalive must be a non-negative number";
            goto cleanup;
        }
        if ((double)item->valueint != item->valuedouble || item->valueint < 0 || item->valueint > UINT16_MAX) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "Invalid keepalive";
            goto cleanup;
        }
        updated.keepalive_seconds = (uint16_t)item->valueint;
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "default_qos");
    if (item != NULL) {
        if (!cJSON_IsNumber(item)) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "default_qos must be a number";
            goto cleanup;
        }
        if ((double)item->valueint != item->valuedouble || item->valueint < 0 || item->valueint > 2) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "default_qos must be between 0 and 2";
            goto cleanup;
        }
        updated.default_qos = (uint8_t)item->valueint;
    }

    item = cJSON_GetObjectItemCaseSensitive(root, "retain");
    if (item != NULL) {
        if (!cJSON_IsBool(item)) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "retain must be a boolean";
            goto cleanup;
        }
        updated.retain_enabled = cJSON_IsTrue(item);
    }

    const cJSON *topics_obj = cJSON_GetObjectItemCaseSensitive(root, "topics");
    if (topics_obj != NULL) {
        if (!cJSON_IsObject(topics_obj)) {
            status = ESP_ERR_INVALID_ARG;
            send_error = true;
            error_message = "topics must be an object";
            goto cleanup;
        }

        const cJSON *topic_item = NULL;
        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "status");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.status must be a string";
                goto cleanup;
            }
            snprintf(topics.status, sizeof(topics.status), "%s", topic_item->valuestring);
        }

        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "metrics");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.metrics must be a string";
                goto cleanup;
            }
            snprintf(topics.metrics, sizeof(topics.metrics), "%s", topic_item->valuestring);
        }

        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "config");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.config must be a string";
                goto cleanup;
            }
            snprintf(topics.config, sizeof(topics.config), "%s", topic_item->valuestring);
        }

        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "can_raw");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.can_raw must be a string";
                goto cleanup;
            }
            snprintf(topics.can_raw, sizeof(topics.can_raw), "%s", topic_item->valuestring);
        }

        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "can_decoded");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.can_decoded must be a string";
                goto cleanup;
            }
            snprintf(topics.can_decoded, sizeof(topics.can_decoded), "%s", topic_item->valuestring);
        }

        topic_item = cJSON_GetObjectItemCaseSensitive(topics_obj, "can_ready");
        if (topic_item != NULL) {
            if (!cJSON_IsString(topic_item) || topic_item->valuestring == NULL) {
                status = ESP_ERR_INVALID_ARG;
                send_error = true;
                error_message = "topics.can_ready must be a string";
                goto cleanup;
            }
            snprintf(topics.can_ready, sizeof(topics.can_ready), "%s", topic_item->valuestring);
        }
    }

    int uri_len = snprintf(updated.broker_uri,
                           sizeof(updated.broker_uri),
                           "%s://%s:%u",
                           (scheme[0] != '\0') ? scheme : "mqtt",
                           host,
                           (unsigned)port);
    if (uri_len < 0 || uri_len >= (int)sizeof(updated.broker_uri)) {
        status = ESP_ERR_INVALID_ARG;
        send_error = true;
        error_message = "Broker URI too long";
        goto cleanup;
    }

    status = config_manager_set_mqtt_client_config(&updated);
    if (status != ESP_OK) {
        send_error = true;
        error_message = "Failed to update MQTT client";
        goto cleanup;
    }

    status = config_manager_set_mqtt_topics(&topics);
    if (status != ESP_OK) {
        send_error = true;
        error_message = "Failed to update MQTT topics";
        goto cleanup;
    }

    httpd_resp_set_type(req, "application/json");
    status = httpd_resp_sendstr(req, "{\"status\":\"updated\"}");
    goto cleanup;

cleanup:
    if (root != NULL) {
        cJSON_Delete(root);
    }
    if (send_error) {
        httpd_resp_send_err(req, error_status, error_message);
    }
    return status;
}
