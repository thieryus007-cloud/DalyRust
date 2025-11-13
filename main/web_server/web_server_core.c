/**
 * @file web_server_core.c
 * @brief Core initialization and utilities for the web server
 *
 * This file contains:
 * - Global variable definitions shared across web server modules
 * - Core utility functions (security headers, JSON sending, formatting)
 * - Public API functions for config snapshot management
 * - Server initialization and deinitialization
 * - Handler registration for all modules (API, WebSocket, OTA, Auth, Static, Alerts)
 *
 * NOTE: Some handlers are still in web_server.c and need to be moved to modules
 * or made non-static. These include:
 * - web_server_api_mqtt_status_handler
 * - web_server_api_mqtt_test_handler
 * - web_server_api_can_status_handler
 * - web_server_api_history_handler
 * - web_server_api_history_files_handler
 * - web_server_api_history_archive_handler
 * - web_server_api_history_download_handler
 * - web_server_api_registers_get_handler
 * - web_server_api_registers_post_handler
 */

#include "web_server.h"
#include "web_server_private.h"

// Standard C headers
#include <stdbool.h>
#include <stdio.h>
#include <string.h>
#include <time.h>

// ESP-IDF headers
#include "esp_err.h"
#include "esp_http_server.h"
#include "esp_log.h"
#include "esp_spiffs.h"
#include "esp_timer.h"
#include "esp_system.h"
#include "nvs.h"

// FreeRTOS headers
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"

// TWAI driver for state enum
#include "driver/twai.h"

// Application modules
#include "app_events.h"
#include "config_manager.h"
#include "event_bus.h"
#include "alert_manager.h"

// Web server modules
#include "web_server_auth.h"
#include "web_server_ota.h"
#include "web_server_api.h"
#include "web_server_websocket.h"
#include "web_server_static.h"
#include "web_server_alerts.h"

// Forward declarations for handlers still in web_server.c
// TODO: Move these to appropriate modules or make non-static
extern esp_err_t web_server_api_mqtt_status_handler(httpd_req_t *req);
extern esp_err_t web_server_api_mqtt_test_handler(httpd_req_t *req);
extern esp_err_t web_server_api_can_status_handler(httpd_req_t *req);
extern esp_err_t web_server_api_history_handler(httpd_req_t *req);
extern esp_err_t web_server_api_history_files_handler(httpd_req_t *req);
extern esp_err_t web_server_api_history_archive_handler(httpd_req_t *req);
extern esp_err_t web_server_api_history_download_handler(httpd_req_t *req);
extern esp_err_t web_server_api_registers_get_handler(httpd_req_t *req);
extern esp_err_t web_server_api_registers_post_handler(httpd_req_t *req);

// =============================================================================
// Global Variable Definitions
// =============================================================================

const char *TAG = "web_server";

// Event bus integration
event_bus_publish_fn_t s_event_publisher = NULL;
event_bus_subscription_handle_t s_event_subscription = NULL;

// HTTP server handle
httpd_handle_t s_httpd = NULL;

// Secret authorizer callback
web_server_secret_authorizer_fn_t s_config_secret_authorizer = NULL;

// =============================================================================
// Utility Functions
// =============================================================================

const char *web_server_twai_state_to_string(twai_state_t state)
{
    switch (state) {
    case TWAI_STATE_STOPPED:
        return "Arrêté";
    case TWAI_STATE_RUNNING:
        return "En marche";
    case TWAI_STATE_BUS_OFF:
        return "Bus-off";
    case TWAI_STATE_RECOVERING:
        return "Récupération";
    default:
        return "Inconnu";
    }
}

void web_server_set_security_headers(httpd_req_t *req)
{
    // Content Security Policy - restrict resource loading to prevent XSS
    httpd_resp_set_hdr(req, "Content-Security-Policy",
                      "default-src 'self'; "
                      "script-src 'self' 'unsafe-inline'; "
                      "style-src 'self' 'unsafe-inline'; "
                      "img-src 'self' data:; "
                      "connect-src 'self' ws: wss:; "
                      "font-src 'self'; "
                      "object-src 'none'; "
                      "base-uri 'self'; "
                      "form-action 'self'");

    // Prevent clickjacking attacks
    httpd_resp_set_hdr(req, "X-Frame-Options", "DENY");

    // Prevent MIME sniffing
    httpd_resp_set_hdr(req, "X-Content-Type-Options", "nosniff");

    // Enable XSS protection in older browsers
    httpd_resp_set_hdr(req, "X-XSS-Protection", "1; mode=block");

    // Referrer policy - don't leak URLs
    httpd_resp_set_hdr(req, "Referrer-Policy", "strict-origin-when-cross-origin");

    // Permissions policy - disable unnecessary features
    httpd_resp_set_hdr(req, "Permissions-Policy",
                      "accelerometer=(), camera=(), geolocation=(), gyroscope=(), "
                      "magnetometer=(), microphone=(), payment=(), usb=()");
}

bool web_server_format_iso8601(time_t timestamp, char *buffer, size_t size)
{
    if (buffer == NULL || size == 0) {
        return false;
    }

    if (timestamp <= 0) {
        buffer[0] = '\0';
        return false;
    }

    struct tm tm_utc;
    if (gmtime_r(&timestamp, &tm_utc) == NULL) {
        buffer[0] = '\0';
        return false;
    }

    size_t written = strftime(buffer, size, "%Y-%m-%dT%H:%M:%SZ", &tm_utc);
    if (written == 0) {
        buffer[0] = '\0';
        return false;
    }

    return true;
}

esp_err_t web_server_send_json(httpd_req_t *req, const char *buffer, size_t length)
{
    if (req == NULL || buffer == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    web_server_set_security_headers(req);
    httpd_resp_set_type(req, "application/json");
    httpd_resp_set_hdr(req, "Cache-Control", "no-store");

    size_t offset = 0U;
    while (offset < length) {
        size_t remaining = length - offset;
        size_t chunk = (remaining > WEB_SERVER_JSON_CHUNK_SIZE) ? WEB_SERVER_JSON_CHUNK_SIZE : remaining;

        esp_err_t err = httpd_resp_send_chunk(req, buffer + offset, chunk);
        if (err != ESP_OK) {
            return err;
        }

        offset += chunk;
    }

    return httpd_resp_send_chunk(req, NULL, 0);
}

// =============================================================================
// Public API Functions
// =============================================================================

/**
 * Helper to check if a query parameter value is truthy
 */
static bool web_server_query_value_truthy(const char *value, size_t length)
{
    if (value == NULL || length == 0U) {
        return false;
    }

    if (length == 1U && value[0] == '1') {
        return true;
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

bool web_server_uri_requests_full_snapshot(const char *uri)
{
    if (uri == NULL) {
        return false;
    }

    const char *query = strchr(uri, '?');
    if (query == NULL || *(++query) == '\0') {
        return false;
    }

    while (*query != '\0') {
        const char *next = strpbrk(query, "&;");
        size_t length = (next != NULL) ? (size_t)(next - query) : strlen(query);
        if (length > 0U) {
            const char *eq = memchr(query, '=', length);
            size_t key_len = (eq != NULL) ? (size_t)(eq - query) : length;
            if (key_len == sizeof("include_secrets") - 1U &&
                strncmp(query, "include_secrets", key_len) == 0) {
                if (eq == NULL) {
                    return true;
                }

                size_t value_len = length - key_len - 1U;
                const char *value = eq + 1;
                return web_server_query_value_truthy(value, value_len);
            }
        }

        if (next == NULL) {
            break;
        }
        query = next + 1;
    }

    return false;
}

esp_err_t web_server_prepare_config_snapshot(const char *uri,
                                             bool authorized_for_secrets,
                                             char *buffer,
                                             size_t buffer_size,
                                             size_t *out_length,
                                             const char **visibility_out)
{
    if (visibility_out != NULL) {
        *visibility_out = NULL;
    }

    bool wants_secrets = web_server_uri_requests_full_snapshot(uri);
    config_manager_snapshot_flags_t flags = CONFIG_MANAGER_SNAPSHOT_PUBLIC;
    const char *visibility = "public";

    if (wants_secrets) {
        if (authorized_for_secrets) {
            flags = CONFIG_MANAGER_SNAPSHOT_INCLUDE_SECRETS;
            visibility = "full";
        } else {
            ESP_LOGW(TAG, "Client requested config secrets without authorization");
        }
    }

    esp_err_t err = config_manager_get_config_json(buffer, buffer_size, out_length, flags);
    if (err == ESP_OK && visibility_out != NULL) {
        *visibility_out = visibility;
    }
    return err;
}

// =============================================================================
// Setter Functions
// =============================================================================

void web_server_set_event_publisher(event_bus_publish_fn_t publisher)
{
    s_event_publisher = publisher;
}

void web_server_set_config_secret_authorizer(web_server_secret_authorizer_fn_t authorizer)
{
    s_config_secret_authorizer = authorizer;
}

// =============================================================================
// Initialization and Registration
// =============================================================================

void web_server_init(void)
{
    // Initialize WebSocket subsystem (mutex and client lists)
    web_server_websocket_init();

#if CONFIG_TINYBMS_WEB_AUTH_BASIC_ENABLE
    // Initialize authentication (credentials, CSRF tokens, rate limiting)
    web_server_auth_init();
    if (!s_basic_auth_enabled) {
        ESP_LOGW(TAG, "HTTP authentication is not available; protected endpoints will reject requests");
    }
#endif

    // Mount SPIFFS for static file serving
    esp_err_t err = web_server_mount_spiffs();
    if (err != ESP_OK) {
        ESP_LOGW(TAG, "Serving static assets from SPIFFS disabled");
    }

    // Configure and start HTTP server
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    config.uri_match_fn = httpd_uri_match_wildcard;
    config.lru_purge_enable = true;

    err = httpd_start(&s_httpd, &config);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to start HTTP server: %s", esp_err_to_name(err));
        return;
    }

    // =========================================================================
    // Register API endpoint handlers
    // =========================================================================

    // Metrics endpoints
    const httpd_uri_t api_metrics_runtime = {
        .uri = "/api/metrics/runtime",
        .method = HTTP_GET,
        .handler = web_server_api_metrics_runtime_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_metrics_runtime);

    const httpd_uri_t api_event_bus_metrics = {
        .uri = "/api/event-bus/metrics",
        .method = HTTP_GET,
        .handler = web_server_api_event_bus_metrics_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_event_bus_metrics);

    const httpd_uri_t api_system_tasks = {
        .uri = "/api/system/tasks",
        .method = HTTP_GET,
        .handler = web_server_api_system_tasks_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_system_tasks);

    const httpd_uri_t api_system_modules = {
        .uri = "/api/system/modules",
        .method = HTTP_GET,
        .handler = web_server_api_system_modules_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_system_modules);

    // System restart endpoint
    const httpd_uri_t api_system_restart = {
        .uri = "/api/system/restart",
        .method = HTTP_POST,
        .handler = web_server_api_restart_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_system_restart);

    // Status endpoint
    const httpd_uri_t api_status = {
        .uri = "/api/status",
        .method = HTTP_GET,
        .handler = web_server_api_status_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_status);

    // Configuration endpoints
    const httpd_uri_t api_config_get = {
        .uri = "/api/config",
        .method = HTTP_GET,
        .handler = web_server_api_config_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_config_get);

    const httpd_uri_t api_config_post = {
        .uri = "/api/config",
        .method = HTTP_POST,
        .handler = web_server_api_config_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_config_post);

#if CONFIG_TINYBMS_WEB_AUTH_BASIC_ENABLE
    // CSRF token endpoint
    const httpd_uri_t api_security_csrf = {
        .uri = "/api/security/csrf",
        .method = HTTP_GET,
        .handler = web_server_api_security_csrf_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_security_csrf);
#endif

    // MQTT configuration endpoints
    const httpd_uri_t api_mqtt_config_get = {
        .uri = "/api/mqtt/config",
        .method = HTTP_GET,
        .handler = web_server_api_mqtt_config_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_mqtt_config_get);

    const httpd_uri_t api_mqtt_config_post = {
        .uri = "/api/mqtt/config",
        .method = HTTP_POST,
        .handler = web_server_api_mqtt_config_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_mqtt_config_post);

    // MQTT status and test endpoints (still in web_server.c)
    const httpd_uri_t api_mqtt_status = {
        .uri = "/api/mqtt/status",
        .method = HTTP_GET,
        .handler = web_server_api_mqtt_status_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_mqtt_status);

    const httpd_uri_t api_mqtt_test = {
        .uri = "/api/mqtt/test",
        .method = HTTP_GET,
        .handler = web_server_api_mqtt_test_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_mqtt_test);

    // CAN status endpoint (still in web_server.c)
    const httpd_uri_t api_can_status = {
        .uri = "/api/can/status",
        .method = HTTP_GET,
        .handler = web_server_api_can_status_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_can_status);

    // History endpoints (still in web_server.c)
    const httpd_uri_t api_history = {
        .uri = "/api/history",
        .method = HTTP_GET,
        .handler = web_server_api_history_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_history);

    const httpd_uri_t api_history_files = {
        .uri = "/api/history/files",
        .method = HTTP_GET,
        .handler = web_server_api_history_files_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_history_files);

    const httpd_uri_t api_history_archive = {
        .uri = "/api/history/archive",
        .method = HTTP_GET,
        .handler = web_server_api_history_archive_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_history_archive);

    const httpd_uri_t api_history_download = {
        .uri = "/api/history/download",
        .method = HTTP_GET,
        .handler = web_server_api_history_download_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_history_download);

    // Registers endpoints (still in web_server.c)
    const httpd_uri_t api_registers_get = {
        .uri = "/api/registers",
        .method = HTTP_GET,
        .handler = web_server_api_registers_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_registers_get);

    const httpd_uri_t api_registers_post = {
        .uri = "/api/registers",
        .method = HTTP_POST,
        .handler = web_server_api_registers_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_registers_post);

    // OTA endpoint
    const httpd_uri_t api_ota_post = {
        .uri = "/api/ota",
        .method = HTTP_POST,
        .handler = web_server_api_ota_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_ota_post);

    // =========================================================================
    // Register Alert API endpoints
    // =========================================================================

    const httpd_uri_t api_alerts_config_get = {
        .uri = "/api/alerts/config",
        .method = HTTP_GET,
        .handler = web_server_api_alerts_config_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_config_get);

    const httpd_uri_t api_alerts_config_post = {
        .uri = "/api/alerts/config",
        .method = HTTP_POST,
        .handler = web_server_api_alerts_config_post_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_config_post);

    const httpd_uri_t api_alerts_active = {
        .uri = "/api/alerts/active",
        .method = HTTP_GET,
        .handler = web_server_api_alerts_active_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_active);

    const httpd_uri_t api_alerts_history = {
        .uri = "/api/alerts/history",
        .method = HTTP_GET,
        .handler = web_server_api_alerts_history_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_history);

    const httpd_uri_t api_alerts_ack = {
        .uri = "/api/alerts/acknowledge",
        .method = HTTP_POST,
        .handler = web_server_api_alerts_acknowledge_all_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_ack);

    const httpd_uri_t api_alerts_ack_id = {
        .uri = "/api/alerts/acknowledge/*",
        .method = HTTP_POST,
        .handler = web_server_api_alerts_acknowledge_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_ack_id);

    const httpd_uri_t api_alerts_stats = {
        .uri = "/api/alerts/statistics",
        .method = HTTP_GET,
        .handler = web_server_api_alerts_statistics_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_stats);

    const httpd_uri_t api_alerts_clear = {
        .uri = "/api/alerts/history",
        .method = HTTP_DELETE,
        .handler = web_server_api_alerts_clear_history_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &api_alerts_clear);

    // =========================================================================
    // Register WebSocket handlers
    // =========================================================================

    const httpd_uri_t telemetry_ws = {
        .uri = "/ws/telemetry",
        .method = HTTP_GET,
        .handler = web_server_telemetry_ws_handler,
        .user_ctx = NULL,
        .is_websocket = true,
    };
    httpd_register_uri_handler(s_httpd, &telemetry_ws);

    const httpd_uri_t events_ws = {
        .uri = "/ws/events",
        .method = HTTP_GET,
        .handler = web_server_events_ws_handler,
        .user_ctx = NULL,
        .is_websocket = true,
    };
    httpd_register_uri_handler(s_httpd, &events_ws);

    const httpd_uri_t uart_ws = {
        .uri = "/ws/uart",
        .method = HTTP_GET,
        .handler = web_server_uart_ws_handler,
        .user_ctx = NULL,
        .is_websocket = true,
    };
    httpd_register_uri_handler(s_httpd, &uart_ws);

    const httpd_uri_t can_ws = {
        .uri = "/ws/can",
        .method = HTTP_GET,
        .handler = web_server_can_ws_handler,
        .user_ctx = NULL,
        .is_websocket = true,
    };
    httpd_register_uri_handler(s_httpd, &can_ws);

    const httpd_uri_t ws_alerts = {
        .uri = "/ws/alerts",
        .method = HTTP_GET,
        .handler = web_server_ws_alerts_handler,
        .user_ctx = NULL,
        .is_websocket = true,
        .handle_ws_control_frames = true,
    };
    httpd_register_uri_handler(s_httpd, &ws_alerts);

    // =========================================================================
    // Register static file handler (fallback, must be last)
    // =========================================================================

    const httpd_uri_t static_files = {
        .uri = "/*",
        .method = HTTP_GET,
        .handler = web_server_static_get_handler,
        .user_ctx = NULL,
    };
    httpd_register_uri_handler(s_httpd, &static_files);

    // =========================================================================
    // Initialize alert manager and event subscription
    // =========================================================================

    alert_manager_init();
    if (s_event_publisher != NULL) {
        alert_manager_set_event_publisher(s_event_publisher);
    }

    s_event_subscription = event_bus_subscribe_default_named("web_server", NULL, NULL);
    if (s_event_subscription == NULL) {
        ESP_LOGW(TAG, "Failed to subscribe to event bus; WebSocket forwarding disabled");
        return;
    }

    // Start WebSocket event task for broadcasting events
    web_server_websocket_start_event_task();
}

void web_server_deinit(void)
{
    ESP_LOGI(TAG, "Deinitializing web server...");

    // Stop WebSocket event task
    web_server_websocket_stop_event_task();

    // Stop HTTP server
    if (s_httpd != NULL) {
        httpd_stop(s_httpd);
        s_httpd = NULL;
        ESP_LOGI(TAG, "HTTP server stopped");
    }

    // Cleanup WebSocket subsystem
    web_server_websocket_deinit();

    // Unsubscribe from event bus
    if (s_event_subscription != NULL) {
        event_bus_unsubscribe(s_event_subscription);
        s_event_subscription = NULL;
    }

    // Unmount SPIFFS (may already be unmounted by config_manager)
    esp_err_t err = esp_vfs_spiffs_unregister(NULL);
    if (err != ESP_OK && err != ESP_ERR_INVALID_STATE) {
        ESP_LOGW(TAG, "Failed to unmount SPIFFS: %s", esp_err_to_name(err));
    }

    // Reset state
    s_event_publisher = NULL;

    ESP_LOGI(TAG, "Web server deinitialized");
}
