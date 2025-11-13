#pragma once

#include "esp_err.h"
#include "esp_http_server.h"
#include "web_server_private.h"

/**
 * @file web_server_api.h
 * @brief REST API endpoint handlers
 */

// Metrics endpoints
esp_err_t web_server_api_metrics_runtime_handler(httpd_req_t *req);
esp_err_t web_server_api_event_bus_metrics_handler(httpd_req_t *req);
esp_err_t web_server_api_system_tasks_handler(httpd_req_t *req);
esp_err_t web_server_api_system_modules_handler(httpd_req_t *req);

// Status endpoint
esp_err_t web_server_api_status_handler(httpd_req_t *req);

// Configuration endpoints
esp_err_t web_server_api_config_get_handler(httpd_req_t *req);
esp_err_t web_server_api_config_post_handler(httpd_req_t *req);

// MQTT configuration endpoints
esp_err_t web_server_api_mqtt_config_get_handler(httpd_req_t *req);
esp_err_t web_server_api_mqtt_config_post_handler(httpd_req_t *req);
