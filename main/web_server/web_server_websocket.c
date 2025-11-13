#include "web_server_websocket.h"
#include "web_server_private.h"

#include <stdlib.h>
#include <string.h>

#include "esp_log.h"
#include "esp_timer.h"
#include "esp_http_server.h"

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/semphr.h"

#include "monitoring.h"
#include "alert_manager.h"
#include "web_server_alerts.h"
#include "app_events.h"
#include "event_bus.h"

// =============================================================================
// Global WebSocket state
// =============================================================================

static const char *TAG = "web_server";

SemaphoreHandle_t s_ws_mutex = NULL;
ws_client_t *s_telemetry_clients = NULL;
ws_client_t *s_event_clients = NULL;
ws_client_t *s_uart_clients = NULL;
ws_client_t *s_can_clients = NULL;
ws_client_t *s_alert_clients = NULL;

event_bus_subscription_handle_t s_event_subscription = NULL;
TaskHandle_t s_event_task_handle = NULL;
volatile bool s_event_task_should_stop = false;

// =============================================================================
// WebSocket client list management
// =============================================================================

/**
 * Free all clients in a WebSocket client list
 * @param list Pointer to the list head pointer
 */
static void ws_client_list_free(ws_client_t **list)
{
    if (list == NULL) {
        return;
    }

    ws_client_t *current = *list;
    while (current != NULL) {
        ws_client_t *next = current->next;
        free(current);
        current = next;
    }
    *list = NULL;
}

static void ws_client_list_add(ws_client_t **list, int fd)
{
    if (list == NULL || fd < 0 || s_ws_mutex == NULL) {
        return;
    }

    if (xSemaphoreTake(s_ws_mutex, pdMS_TO_TICKS(50)) != pdTRUE) {
        return;
    }

    for (ws_client_t *iter = *list; iter != NULL; iter = iter->next) {
        if (iter->fd == fd) {
            xSemaphoreGive(s_ws_mutex);
            return;
        }
    }

    ws_client_t *client = calloc(1, sizeof(ws_client_t));
    if (client == NULL) {
        xSemaphoreGive(s_ws_mutex);
        ESP_LOGW(TAG, "Unable to allocate memory for websocket client");
        return;
    }

    client->fd = fd;
    client->next = *list;
    // Initialize rate limiting (calloc already zeroed message_count and total_violations)
    client->last_reset_time = esp_timer_get_time() / 1000;  // Convert to ms
    *list = client;

    xSemaphoreGive(s_ws_mutex);
}

static void ws_client_list_remove(ws_client_t **list, int fd)
{
    if (list == NULL || s_ws_mutex == NULL) {
        return;
    }

    if (xSemaphoreTake(s_ws_mutex, pdMS_TO_TICKS(50)) != pdTRUE) {
        return;
    }

    ws_client_t *prev = NULL;
    ws_client_t *iter = *list;
    while (iter != NULL) {
        if (iter->fd == fd) {
            if (prev == NULL) {
                *list = iter->next;
            } else {
                prev->next = iter->next;
            }
            free(iter);
            break;
        }
        prev = iter;
        iter = iter->next;
    }

    xSemaphoreGive(s_ws_mutex);
}

static void ws_client_list_broadcast(ws_client_t **list, const char *payload, size_t length)
{
    if (list == NULL || payload == NULL || length == 0 || s_ws_mutex == NULL || s_httpd == NULL) {
        return;
    }

    // Validate payload size to prevent DoS attacks
    if (length > WEB_SERVER_WS_MAX_PAYLOAD_SIZE) {
        ESP_LOGW(TAG, "WebSocket broadcast: payload too large (%zu bytes > %d max), dropping",
                 length, WEB_SERVER_WS_MAX_PAYLOAD_SIZE);
        return;
    }

    // Calculer la longueur du payload (sans le '\0' final si présent)
    size_t payload_length = length;
    if (payload_length > 0 && payload[payload_length - 1] == '\0') {
        payload_length -= 1;
    }

    if (payload_length == 0) {
        return;
    }

    // Copier la liste des FDs sous mutex pour minimiser la section critique
    #define MAX_BROADCAST_CLIENTS 32
    int client_fds[MAX_BROADCAST_CLIENTS];
    size_t client_count = 0;

    if (xSemaphoreTake(s_ws_mutex, pdMS_TO_TICKS(50)) != pdTRUE) {
        ESP_LOGW(TAG, "WebSocket broadcast: failed to acquire mutex (timeout), event dropped");
        return;
    }

    int64_t current_time = esp_timer_get_time() / 1000;  // Convert to ms
    ws_client_t *iter = *list;
    while (iter != NULL && client_count < MAX_BROADCAST_CLIENTS) {
        // Check and update rate limiting
        int64_t time_since_reset = current_time - iter->last_reset_time;

        // Reset rate window if expired
        if (time_since_reset >= WEB_SERVER_WS_RATE_WINDOW_MS) {
            iter->last_reset_time = current_time;
            iter->message_count = 0;
        }

        // Check rate limit
        if (iter->message_count >= WEB_SERVER_WS_MAX_MSGS_PER_SEC) {
            iter->total_violations++;
            if (iter->total_violations % 10 == 1) {  // Log every 10th violation to avoid spam
                ESP_LOGW(TAG, "WebSocket client fd=%d rate limited (%u msgs in window, %u total violations)",
                         iter->fd, iter->message_count, iter->total_violations);
            }
            iter = iter->next;
            continue;  // Skip this client
        }

        // Client is within rate limit, include in broadcast
        iter->message_count++;
        client_fds[client_count++] = iter->fd;
        iter = iter->next;
    }

    xSemaphoreGive(s_ws_mutex);

    // Diffuser hors section critique pour éviter les blocages
    httpd_ws_frame_t frame = {
        .final = true,
        .fragmented = false,
        .type = HTTPD_WS_TYPE_TEXT,
        .payload = (uint8_t *)payload,
        .len = payload_length,
    };

    for (size_t i = 0; i < client_count; i++) {
        esp_err_t err = httpd_ws_send_frame_async(s_httpd, client_fds[i], &frame);
        if (err != ESP_OK) {
            ESP_LOGW(TAG, "Failed to send to websocket client %d: %s", client_fds[i], esp_err_to_name(err));
            // Retirer le client en échec de la liste
            ws_client_list_remove(list, client_fds[i]);
        }
    }
}

static void web_server_broadcast_battery_snapshot(ws_client_t **list, const char *payload, size_t length)
{
    if (list == NULL || payload == NULL || length == 0) {
        return;
    }

    size_t payload_length = length;
    if (payload_length > 0U && payload[payload_length - 1U] == '\0') {
        payload_length -= 1U;
    }

    if (payload_length == 0U) {
        return;
    }

    if (payload_length >= MONITORING_SNAPSHOT_MAX_SIZE) {
        ESP_LOGW(TAG, "Telemetry snapshot too large to wrap (%zu bytes)", payload_length);
        return;
    }

    char wrapped[MONITORING_SNAPSHOT_MAX_SIZE + 32U];
    int written = snprintf(wrapped, sizeof(wrapped), "{\"battery\":%.*s}", (int)payload_length, payload);
    if (written <= 0 || (size_t)written >= sizeof(wrapped)) {
        ESP_LOGW(TAG, "Failed to wrap telemetry snapshot for broadcast");
        return;
    }

    ws_client_list_broadcast(list, wrapped, (size_t)written);
}

// =============================================================================
// WebSocket protocol handlers
// =============================================================================

static esp_err_t web_server_handle_ws_close(httpd_req_t *req, ws_client_t **list)
{
    int fd = httpd_req_to_sockfd(req);
    ws_client_list_remove(list, fd);
    ESP_LOGI(TAG, "WebSocket client %d disconnected", fd);
    return ESP_OK;
}

static esp_err_t web_server_ws_control_frame(httpd_req_t *req, httpd_ws_frame_t *frame)
{
    if (frame->type == HTTPD_WS_TYPE_PING) {
        httpd_ws_frame_t response = {
            .final = true,
            .fragmented = false,
            .type = HTTPD_WS_TYPE_PONG,
            .payload = frame->payload,
            .len = frame->len,
        };
        return httpd_ws_send_frame(req, &response);
    }

    if (frame->type == HTTPD_WS_TYPE_CLOSE) {
        return ESP_OK;
    }

    return ESP_OK;
}

static esp_err_t web_server_ws_receive(httpd_req_t *req, ws_client_t **list)
{
    httpd_ws_frame_t frame = {
        .type = HTTPD_WS_TYPE_TEXT,
        .payload = NULL,
    };

    esp_err_t err = httpd_ws_recv_frame(req, &frame, 0);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "Failed to get frame length: %s", esp_err_to_name(err));
        return err;
    }

    // Validate incoming payload size to prevent DoS attacks
    if (frame.len > WEB_SERVER_WS_MAX_PAYLOAD_SIZE) {
        ESP_LOGW(TAG, "WebSocket receive: payload too large (%zu bytes > %d max), rejecting",
                 frame.len, WEB_SERVER_WS_MAX_PAYLOAD_SIZE);
        return ESP_ERR_INVALID_SIZE;
    }

    if (frame.len > 0) {
        frame.payload = calloc(1, frame.len + 1);
        if (frame.payload == NULL) {
            return ESP_ERR_NO_MEM;
        }
        err = httpd_ws_recv_frame(req, &frame, frame.len);
        if (err != ESP_OK) {
            free(frame.payload);
            ESP_LOGE(TAG, "Failed to read frame payload: %s", esp_err_to_name(err));
            return err;
        }
    }

    if (frame.type == HTTPD_WS_TYPE_CLOSE) {
        free(frame.payload);
        return web_server_handle_ws_close(req, list);
    }

    err = web_server_ws_control_frame(req, &frame);
    if (err != ESP_OK) {
        free(frame.payload);
        return err;
    }

    if (frame.type == HTTPD_WS_TYPE_TEXT && frame.payload != NULL) {
        ESP_LOGD(TAG, "WS message: %.*s", frame.len, frame.payload);
    }

    free(frame.payload);
    return ESP_OK;
}

// =============================================================================
// WebSocket endpoint handlers
// =============================================================================

esp_err_t web_server_telemetry_ws_handler(httpd_req_t *req)
{
    if (req->method == HTTP_GET) {
        int fd = httpd_req_to_sockfd(req);
        ws_client_list_add(&s_telemetry_clients, fd);
        ESP_LOGI(TAG, "Telemetry WebSocket client connected: %d", fd);

        char buffer[MONITORING_SNAPSHOT_MAX_SIZE];
        size_t length = 0;
        if (monitoring_get_status_json(buffer, sizeof(buffer), &length) == ESP_OK) {
            httpd_ws_frame_t frame = {
                .final = true,
                .fragmented = false,
                .type = HTTPD_WS_TYPE_TEXT,
                .payload = (uint8_t *)buffer,
                .len = length,
            };
            httpd_ws_send_frame(req, &frame);
        }

        return ESP_OK;
    }

    return web_server_ws_receive(req, &s_telemetry_clients);
}

esp_err_t web_server_events_ws_handler(httpd_req_t *req)
{
    if (req->method == HTTP_GET) {
        int fd = httpd_req_to_sockfd(req);
        ws_client_list_add(&s_event_clients, fd);
        ESP_LOGI(TAG, "Events WebSocket client connected: %d", fd);

        static const char k_ready_message[] = "{\"event\":\"connected\"}";
        httpd_ws_frame_t frame = {
            .final = true,
            .fragmented = false,
            .type = HTTPD_WS_TYPE_TEXT,
            .payload = (uint8_t *)k_ready_message,
            .len = sizeof(k_ready_message) - 1,
        };
        httpd_ws_send_frame(req, &frame);
        return ESP_OK;
    }

    return web_server_ws_receive(req, &s_event_clients);
}

esp_err_t web_server_uart_ws_handler(httpd_req_t *req)
{
    if (req->method == HTTP_GET) {
        int fd = httpd_req_to_sockfd(req);
        ws_client_list_add(&s_uart_clients, fd);
        ESP_LOGI(TAG, "UART WebSocket client connected: %d", fd);

        static const char k_ready_message[] = "{\"type\":\"uart\",\"status\":\"connected\"}";
        httpd_ws_frame_t frame = {
            .final = true,
            .fragmented = false,
            .type = HTTPD_WS_TYPE_TEXT,
            .payload = (uint8_t *)k_ready_message,
            .len = sizeof(k_ready_message) - 1,
        };
        httpd_ws_send_frame(req, &frame);
        return ESP_OK;
    }

    return web_server_ws_receive(req, &s_uart_clients);
}

esp_err_t web_server_can_ws_handler(httpd_req_t *req)
{
    if (req->method == HTTP_GET) {
        int fd = httpd_req_to_sockfd(req);
        ws_client_list_add(&s_can_clients, fd);
        ESP_LOGI(TAG, "CAN WebSocket client connected: %d", fd);

        static const char k_ready_message[] = "{\"type\":\"can\",\"status\":\"connected\"}";
        httpd_ws_frame_t frame = {
            .final = true,
            .fragmented = false,
            .type = HTTPD_WS_TYPE_TEXT,
            .payload = (uint8_t *)k_ready_message,
            .len = sizeof(k_ready_message) - 1,
        };
        httpd_ws_send_frame(req, &frame);
        return ESP_OK;
    }

    return web_server_ws_receive(req, &s_can_clients);
}

// =============================================================================
// Event dispatcher task
// =============================================================================

static void web_server_event_task(void *context)
{
    TaskHandle_t parent_task = (TaskHandle_t)context;

    if (s_event_subscription == NULL) {
        s_event_task_handle = NULL;
        if (parent_task != NULL) {
            xTaskNotifyGive(parent_task);
        }
        vTaskDelete(NULL);
        return;
    }

    event_bus_event_t event = {0};
    while (!s_event_task_should_stop) {
        // Utiliser un timeout pour permettre la vérification périodique du drapeau de terminaison
        if (!event_bus_receive(s_event_subscription, &event, pdMS_TO_TICKS(1000))) {
            continue; // Timeout, vérifier le drapeau et réessayer
        }

        const char *payload = NULL;
        size_t length = 0U;
        char generated_payload[WEB_SERVER_EVENT_BUS_JSON_SIZE];

        if (event.payload != NULL && event.payload_size == sizeof(app_event_metadata_t)) {
            const app_event_metadata_t *metadata = (const app_event_metadata_t *)event.payload;
            if (metadata->event_id == event.id) {
                const char *key = (metadata->key != NULL) ? metadata->key : "";
                const char *type = (metadata->type != NULL) ? metadata->type : "";
                const char *label = (metadata->label != NULL) ? metadata->label : "";
                unsigned long long timestamp = (unsigned long long)metadata->timestamp_ms;
                int written = snprintf(generated_payload,
                                       sizeof(generated_payload),
                                       "{\"event_id\":%u,\"key\":\"%s\",\"type\":\"%s\",\"timestamp\":%llu",
                                       (unsigned)metadata->event_id,
                                       key,
                                       type,
                                       timestamp);
                if (written > 0 && (size_t)written < sizeof(generated_payload)) {
                    size_t used = (size_t)written;
                    if (label[0] != '\0' && used < sizeof(generated_payload)) {
                        int appended = snprintf(generated_payload + used,
                                                sizeof(generated_payload) - used,
                                                ",\"label\":\"%s\"",
                                                label);
                        if (appended > 0 && (size_t)appended < sizeof(generated_payload) - used) {
                            used += (size_t)appended;
                        }
                    }
                    if (used < sizeof(generated_payload)) {
                        int closed = snprintf(generated_payload + used,
                                              sizeof(generated_payload) - used,
                                              "}");
                        if (closed > 0 && (size_t)closed < sizeof(generated_payload) - used) {
                            used += (size_t)closed;
                            payload = generated_payload;
                            length = used;
                        }
                    }
                }
            }
        } else if (event.payload != NULL && event.payload_size > 0U) {
            payload = (const char *)event.payload;
            length = event.payload_size;
            if (length > 0U && payload[length - 1U] == '\0') {
                length -= 1U;
            }
        } else {
            int written = snprintf(generated_payload,
                                   sizeof(generated_payload),
                                   "{\"event_id\":%u}",
                                   (unsigned)event.id);
            if (written > 0 && (size_t)written < sizeof(generated_payload)) {
                payload = generated_payload;
                length = (size_t)written;
            }
        }

        if (payload == NULL || length == 0U) {
            continue;
        }

        switch (event.id) {
        case APP_EVENT_ID_TELEMETRY_SAMPLE:
            web_server_broadcast_battery_snapshot(&s_telemetry_clients, payload, length);
            break;
        case APP_EVENT_ID_UI_NOTIFICATION:
        case APP_EVENT_ID_CONFIG_UPDATED:
        case APP_EVENT_ID_OTA_UPLOAD_READY:
        case APP_EVENT_ID_MONITORING_DIAGNOSTICS:
            ws_client_list_broadcast(&s_event_clients, payload, length);
            break;
        case APP_EVENT_ID_WIFI_STA_START:
        case APP_EVENT_ID_WIFI_STA_CONNECTED:
        case APP_EVENT_ID_WIFI_STA_DISCONNECTED:
        case APP_EVENT_ID_WIFI_STA_GOT_IP:
        case APP_EVENT_ID_WIFI_STA_LOST_IP:
        case APP_EVENT_ID_WIFI_AP_STARTED:
        case APP_EVENT_ID_WIFI_AP_STOPPED:
        case APP_EVENT_ID_WIFI_AP_CLIENT_CONNECTED:
        case APP_EVENT_ID_WIFI_AP_CLIENT_DISCONNECTED:
        case APP_EVENT_ID_STORAGE_HISTORY_READY:
        case APP_EVENT_ID_STORAGE_HISTORY_UNAVAILABLE:
            ws_client_list_broadcast(&s_event_clients, payload, length);
            break;
        case APP_EVENT_ID_UART_FRAME_RAW:
        case APP_EVENT_ID_UART_FRAME_DECODED:
            ws_client_list_broadcast(&s_uart_clients, payload, length);
            break;
        case APP_EVENT_ID_CAN_FRAME_RAW:
        case APP_EVENT_ID_CAN_FRAME_DECODED:
            ws_client_list_broadcast(&s_can_clients, payload, length);
            break;
        case APP_EVENT_ID_ALERT_TRIGGERED:
            ws_client_list_broadcast(&s_alert_clients, payload, length);
            break;
        default:
            break;
        }
    }

    ESP_LOGI(TAG, "Event task shutting down cleanly");
    s_event_task_handle = NULL;

    // Notify parent task that we're done
    if (parent_task != NULL) {
        xTaskNotifyGive(parent_task);
    }

    vTaskDelete(NULL);
}

// =============================================================================
// Initialization and cleanup
// =============================================================================

void web_server_websocket_init(void)
{
    if (s_ws_mutex == NULL) {
        s_ws_mutex = xSemaphoreCreateMutex();
    }

    if (s_ws_mutex == NULL) {
        ESP_LOGE(TAG, "Failed to create websocket mutex");
        return;
    }

    // Initialize client lists (already NULL from global initialization)
    s_telemetry_clients = NULL;
    s_event_clients = NULL;
    s_uart_clients = NULL;
    s_can_clients = NULL;
    s_alert_clients = NULL;

    ESP_LOGI(TAG, "WebSocket subsystem initialized");
}

void web_server_websocket_deinit(void)
{
    ESP_LOGI(TAG, "Deinitializing WebSocket subsystem...");

    // Free all WebSocket client lists
    if (s_ws_mutex != NULL) {
        if (xSemaphoreTake(s_ws_mutex, pdMS_TO_TICKS(WEB_SERVER_MUTEX_TIMEOUT_MS)) == pdTRUE) {
            ws_client_list_free(&s_telemetry_clients);
            ws_client_list_free(&s_event_clients);
            ws_client_list_free(&s_uart_clients);
            ws_client_list_free(&s_can_clients);
            ws_client_list_free(&s_alert_clients);
            xSemaphoreGive(s_ws_mutex);
        } else {
            ESP_LOGW(TAG, "Failed to acquire WS mutex for cleanup (timeout)");
        }
    }

    // Unsubscribe from event bus
    if (s_event_subscription != NULL) {
        event_bus_unsubscribe(s_event_subscription);
        s_event_subscription = NULL;
    }

    // Destroy websocket mutex
    if (s_ws_mutex != NULL) {
        vSemaphoreDelete(s_ws_mutex);
        s_ws_mutex = NULL;
    }

    ESP_LOGI(TAG, "WebSocket subsystem deinitialized");
}

void web_server_websocket_start_event_task(void)
{
    if (s_event_subscription != NULL) {
        ESP_LOGW(TAG, "Event task already running or subscription already active");
        return;
    }

    s_event_subscription = event_bus_subscribe_default_named("web_server", NULL, NULL);
    if (s_event_subscription == NULL) {
        ESP_LOGW(TAG, "Failed to subscribe to event bus; WebSocket forwarding disabled");
        return;
    }

    // Reset the stop flag
    s_event_task_should_stop = false;

    // Pass current task handle so event task can notify us when it exits
    TaskHandle_t current_task = xTaskGetCurrentTaskHandle();
    if (xTaskCreate(web_server_event_task, "ws_event", 4096, (void *)current_task, 5, &s_event_task_handle) != pdPASS) {
        ESP_LOGE(TAG, "Failed to start event dispatcher task");
        event_bus_unsubscribe(s_event_subscription);
        s_event_subscription = NULL;
    } else {
        ESP_LOGI(TAG, "WebSocket event task started");
    }
}

void web_server_websocket_stop_event_task(void)
{
    ESP_LOGI(TAG, "Stopping WebSocket event task...");

    // Signal event task to exit
    s_event_task_should_stop = true;

    // Wait for event task to exit cleanly (max 5 seconds)
    if (s_event_task_handle != NULL) {
        ESP_LOGI(TAG, "Waiting for event task to exit...");
        if (ulTaskNotifyTake(pdTRUE, pdMS_TO_TICKS(5000)) == 0) {
            ESP_LOGW(TAG, "Event task did not exit within timeout");
        } else {
            ESP_LOGI(TAG, "Event task exited cleanly");
        }
    }

    // Unsubscribe from event bus
    if (s_event_subscription != NULL) {
        event_bus_unsubscribe(s_event_subscription);
        s_event_subscription = NULL;
    }

    // Reset state
    s_event_task_handle = NULL;
    s_event_task_should_stop = false;

    ESP_LOGI(TAG, "WebSocket event task stopped");
}
