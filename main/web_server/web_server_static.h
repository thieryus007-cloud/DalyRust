#pragma once

#include "esp_err.h"
#include "esp_http_server.h"
#include "web_server_private.h"

/**
 * @file web_server_static.h
 * @brief Static file serving from SPIFFS
 */

/**
 * Mount SPIFFS filesystem
 * @return ESP_OK on success
 */
esp_err_t web_server_mount_spiffs(void);

/**
 * Handler for GET /* - serve static files from SPIFFS
 * @param req HTTP request
 * @return ESP_OK on success
 */
esp_err_t web_server_static_get_handler(httpd_req_t *req);
