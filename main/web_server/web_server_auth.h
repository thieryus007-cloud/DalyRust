#pragma once

#include <stdbool.h>
#include "esp_err.h"
#include "esp_http_server.h"
#include "web_server_private.h"

/**
 * @file web_server_auth.h
 * @brief HTTP Basic Authentication and CSRF token management
 */

/**
 * Initialize authentication subsystem
 * - Load credentials from NVS or provision defaults
 * - Initialize CSRF token storage
 * - Setup rate limiting for brute-force protection
 */
void web_server_auth_init(void);

/**
 * Validate HTTP Basic Authentication and optionally CSRF token
 * @param req HTTP request
 * @param require_csrf Whether to require X-CSRF-Token header
 * @param out_username Buffer to store authenticated username (optional)
 * @param out_size Size of username buffer
 * @return true if authorized, false otherwise (response already sent)
 */
bool web_server_require_authorization(httpd_req_t *req, bool require_csrf, char *out_username, size_t out_size);

/**
 * Check if request is authorized to view secrets
 * @param req HTTP request
 * @return true if authorized for secrets
 */
bool web_server_request_authorized_for_secrets(httpd_req_t *req);

/**
 * Send 401 Unauthorized response with WWW-Authenticate header
 * @param req HTTP request
 */
void web_server_send_unauthorized(httpd_req_t *req);

/**
 * Send 403 Forbidden response with error message
 * @param req HTTP request
 * @param message Error message (optional, defaults to "forbidden")
 */
void web_server_send_forbidden(httpd_req_t *req, const char *message);

/**
 * Handler for GET /api/security/csrf - issue new CSRF token
 * @param req HTTP request
 * @return ESP_OK on success
 */
esp_err_t web_server_api_security_csrf_get_handler(httpd_req_t *req);
