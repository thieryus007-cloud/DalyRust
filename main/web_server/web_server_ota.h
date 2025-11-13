#pragma once

#include "esp_err.h"
#include "esp_http_server.h"
#include "web_server_private.h"

/**
 * @file web_server_ota.h
 * @brief OTA firmware upload and system restart handlers
 */

/**
 * Handler for POST /api/ota - firmware upload via multipart/form-data
 * @param req HTTP request with multipart firmware file
 * @return ESP_OK on success
 */
esp_err_t web_server_api_ota_post_handler(httpd_req_t *req);

/**
 * Handler for POST /api/system/restart - system restart
 * @param req HTTP request with restart target JSON body
 * @return ESP_OK on success
 */
esp_err_t web_server_api_restart_post_handler(httpd_req_t *req);
