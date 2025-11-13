#pragma once

#include "esp_err.h"
#include "esp_http_server.h"
#include "web_server_private.h"

/**
 * @file web_server_websocket.h
 * @brief WebSocket management and event broadcasting
 */

/**
 * Initialize WebSocket subsystem (mutex and client lists)
 */
void web_server_websocket_init(void);

/**
 * Cleanup WebSocket subsystem (free all clients and mutex)
 */
void web_server_websocket_deinit(void);

/**
 * Start WebSocket event task
 */
void web_server_websocket_start_event_task(void);

/**
 * Stop WebSocket event task
 */
void web_server_websocket_stop_event_task(void);

// WebSocket handlers
esp_err_t web_server_telemetry_ws_handler(httpd_req_t *req);
esp_err_t web_server_events_ws_handler(httpd_req_t *req);
esp_err_t web_server_uart_ws_handler(httpd_req_t *req);
esp_err_t web_server_can_ws_handler(httpd_req_t *req);
