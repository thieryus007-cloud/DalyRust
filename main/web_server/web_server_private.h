#pragma once

#include <stdbool.h>
#include <stdint.h>
#include "esp_err.h"
#include "esp_http_server.h"
#include "freertos/FreeRTOS.h"
#include "freertos/semphr.h"
#include "freertos/task.h"
#include "event_bus.h"
#include "app_events.h"
#include "web_server.h"

/**
 * @file web_server_private.h
 * @brief Shared internal structures, constants, and state for web_server modules
 */

// HTTP status codes not defined in ESP-IDF
#ifndef HTTPD_413_PAYLOAD_TOO_LARGE
#define HTTPD_413_PAYLOAD_TOO_LARGE 413
#endif

#ifndef HTTPD_414_URI_TOO_LONG
#define HTTPD_414_URI_TOO_LONG 414
#endif

#ifndef HTTPD_503_SERVICE_UNAVAILABLE
#define HTTPD_503_SERVICE_UNAVAILABLE 503
#endif

#ifndef HTTPD_415_UNSUPPORTED_MEDIA_TYPE
#define HTTPD_415_UNSUPPORTED_MEDIA_TYPE 415
#endif

#ifndef HTTPD_401_UNAUTHORIZED
#define HTTPD_401_UNAUTHORIZED 401
#endif

#ifndef HTTPD_403_FORBIDDEN
#define HTTPD_403_FORBIDDEN 403
#endif

// File system and paths
#define WEB_SERVER_FS_BASE_PATH "/spiffs"
#define WEB_SERVER_WEB_ROOT     WEB_SERVER_FS_BASE_PATH
#define WEB_SERVER_INDEX_PATH   WEB_SERVER_WEB_ROOT "/index.html"
#define WEB_SERVER_MAX_PATH     256
#define WEB_SERVER_FILE_BUFSZ   1024

// Multipart upload
#define WEB_SERVER_MULTIPART_BUFFER_SIZE 2048
#define WEB_SERVER_MULTIPART_BOUNDARY_MAX 72
#define WEB_SERVER_MULTIPART_HEADER_MAX 256

// System control
#define WEB_SERVER_RESTART_DEFAULT_DELAY_MS 750U

// JSON buffer sizes
#define WEB_SERVER_HISTORY_JSON_SIZE      4096
#define WEB_SERVER_MQTT_JSON_SIZE         768
#define WEB_SERVER_CAN_JSON_SIZE          512
#define WEB_SERVER_RUNTIME_JSON_SIZE      1536
#define WEB_SERVER_EVENT_BUS_JSON_SIZE    1536
#define WEB_SERVER_TASKS_JSON_SIZE        8192
#define WEB_SERVER_MODULES_JSON_SIZE      2048
#define WEB_SERVER_JSON_CHUNK_SIZE        1024

// Authentication constants
#define WEB_SERVER_AUTH_NAMESPACE              "web_auth"
#define WEB_SERVER_AUTH_USERNAME_KEY           "username"
#define WEB_SERVER_AUTH_SALT_KEY               "salt"
#define WEB_SERVER_AUTH_HASH_KEY               "password_hash"
#define WEB_SERVER_AUTH_MAX_USERNAME_LENGTH    32
#define WEB_SERVER_AUTH_MAX_PASSWORD_LENGTH    64
#define WEB_SERVER_AUTH_SALT_SIZE              16
#define WEB_SERVER_AUTH_HASH_SIZE              32
#define WEB_SERVER_AUTH_HEADER_MAX             192
#define WEB_SERVER_MUTEX_TIMEOUT_MS            5000  // Timeout 5s pour éviter deadlock
#define WEB_SERVER_AUTH_DECODED_MAX            96

// CSRF token constants
#define WEB_SERVER_CSRF_TOKEN_SIZE             32
#define WEB_SERVER_CSRF_TOKEN_STRING_LENGTH    (WEB_SERVER_CSRF_TOKEN_SIZE * 2)
#define WEB_SERVER_CSRF_TOKEN_TTL_US           (15ULL * 60ULL * 1000000ULL)
#define WEB_SERVER_MAX_CSRF_TOKENS             8

// WebSocket rate limiting and security
#define WEB_SERVER_WS_MAX_PAYLOAD_SIZE    (32 * 1024)  // 32KB max payload
#define WEB_SERVER_WS_MAX_MSGS_PER_SEC    10           // Max 10 messages/sec per client
#define WEB_SERVER_WS_RATE_WINDOW_MS      1000         // 1 second rate limiting window

/**
 * WebSocket client node with rate limiting
 */
typedef struct ws_client {
    int fd;
    struct ws_client *next;
    // Rate limiting
    int64_t last_reset_time;      // Timestamp (ms) of rate window start
    uint32_t message_count;        // Messages sent in current window
    uint32_t total_violations;     // Total rate limit violations
} ws_client_t;

/**
 * CSRF token entry
 */
typedef struct {
    bool in_use;
    char username[WEB_SERVER_AUTH_MAX_USERNAME_LENGTH + 1];
    char token[WEB_SERVER_CSRF_TOKEN_STRING_LENGTH + 1];
    int64_t expires_at_us;
} web_server_csrf_token_t;

/**
 * Multipart form headers
 */
typedef struct {
    char name[WEB_SERVER_MULTIPART_HEADER_MAX];
    char filename[WEB_SERVER_MULTIPART_HEADER_MAX];
    char content_type[WEB_SERVER_MULTIPART_HEADER_MAX];
} web_server_multipart_headers_t;

// =============================================================================
// Shared global state (extern declarations)
// =============================================================================

extern const char *TAG;

// Event bus integration
extern event_bus_publish_fn_t s_event_publisher;
extern event_bus_subscription_handle_t s_event_subscription;
extern TaskHandle_t s_event_task_handle;
extern volatile bool s_event_task_should_stop;

// HTTP server handle
extern httpd_handle_t s_httpd;

// Secret authorizer callback
extern web_server_secret_authorizer_fn_t s_config_secret_authorizer;

// Authentication state (from web_server_auth.c)
extern bool s_basic_auth_enabled;
extern char s_basic_auth_username[WEB_SERVER_AUTH_MAX_USERNAME_LENGTH + 1];
extern uint8_t s_basic_auth_salt[WEB_SERVER_AUTH_SALT_SIZE];
extern uint8_t s_basic_auth_hash[WEB_SERVER_AUTH_HASH_SIZE];
extern SemaphoreHandle_t s_auth_mutex;
extern web_server_csrf_token_t s_csrf_tokens[WEB_SERVER_MAX_CSRF_TOKENS];

// WebSocket state (from web_server_websocket.c)
extern SemaphoreHandle_t s_ws_mutex;
extern ws_client_t *s_telemetry_clients;
extern ws_client_t *s_event_clients;
extern ws_client_t *s_uart_clients;
extern ws_client_t *s_can_clients;
extern ws_client_t *s_alert_clients;

// OTA/restart event metadata (from web_server_ota.c)
extern char s_ota_event_label[128];
extern app_event_metadata_t s_ota_event_metadata;
extern char s_restart_event_label[128];
extern app_event_metadata_t s_restart_event_metadata;

// =============================================================================
// Shared utility functions
// =============================================================================

/**
 * Set security headers on HTTP response
 */
void web_server_set_security_headers(httpd_req_t *req);

/**
 * Format timestamp as ISO8601 string
 */
bool web_server_format_iso8601(time_t timestamp, char *buffer, size_t size);

/**
 * Send JSON response in chunks
 */
esp_err_t web_server_send_json(httpd_req_t *req, const char *buffer, size_t length);

/**
 * Set HTTP status code on response
 */
void web_server_set_http_status_code(httpd_req_t *req, int status_code);

/**
 * Convert TWAI state to string
 */
const char *web_server_twai_state_to_string(twai_state_t state);
