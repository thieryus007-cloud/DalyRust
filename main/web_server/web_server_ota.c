#include "web_server_ota.h"
#include "web_server_auth.h"

#include <inttypes.h>
#include <stdbool.h>
#include <stdint.h>
#include <string.h>
#include <strings.h>

#include "esp_log.h"
#include "esp_timer.h"

#include "cJSON.h"

#include "config_manager.h"
#include "monitoring.h"
#include "ota_update.h"
#include "system_control.h"
#include "web_server_ota_errors.h"

// OTA/restart event metadata (defined here, externed in web_server_private.h)
char s_ota_event_label[128];
app_event_metadata_t s_ota_event_metadata = {
    .event_id = APP_EVENT_ID_OTA_UPLOAD_READY,
    .key = "ota_ready",
    .type = "ota",
    .label = s_ota_event_label,
    .timestamp_ms = 0U,
};
char s_restart_event_label[128];
app_event_metadata_t s_restart_event_metadata = {
    .event_id = APP_EVENT_ID_UI_NOTIFICATION,
    .key = "system_restart",
    .type = "system",
    .label = s_restart_event_label,
    .timestamp_ms = 0U,
};

static void web_server_set_http_status_code(httpd_req_t *req, int status_code)
{
    if (req == NULL) {
        return;
    }

    const char *status = "200 OK";
    switch (status_code) {
    case 200:
        status = "200 OK";
        break;
    case 400:
        status = "400 Bad Request";
        break;
    case 413:
        status = "413 Payload Too Large";
        break;
    case 415:
        status = "415 Unsupported Media Type";
        break;
    case 503:
        status = "503 Service Unavailable";
        break;
    default:
        status = "500 Internal Server Error";
        break;
    }

    httpd_resp_set_status(req, status);
}

static esp_err_t web_server_send_ota_response(httpd_req_t *req,
                                              web_server_ota_error_code_t code,
                                              const char *message_override,
                                              cJSON *data)
{
    if (req == NULL) {
        if (data != NULL) {
            cJSON_Delete(data);
        }
        return ESP_ERR_INVALID_ARG;
    }

    cJSON *root = cJSON_CreateObject();
    if (root == NULL) {
        if (data != NULL) {
            cJSON_Delete(data);
        }
        return ESP_ERR_NO_MEM;
    }

    if (!web_server_ota_set_response_fields(root, code, message_override)) {
        cJSON_Delete(root);
        if (data != NULL) {
            cJSON_Delete(data);
        }
        return ESP_ERR_NO_MEM;
    }

    if (data != NULL) {
        cJSON_AddItemToObject(root, "data", data);
    }

    char *json = cJSON_PrintUnformatted(root);
    cJSON_Delete(root);
    if (json == NULL) {
        return ESP_ERR_NO_MEM;
    }

    size_t length = strlen(json);
    web_server_set_http_status_code(req, web_server_ota_http_status(code));
    esp_err_t err = web_server_send_json(req, json, length);
    cJSON_free(json);
    return err;
}

static const uint8_t *web_server_memmem(const uint8_t *haystack,
                                        size_t haystack_len,
                                        const uint8_t *needle,
                                        size_t needle_len)
{
    if (haystack == NULL || needle == NULL || needle_len == 0 || haystack_len < needle_len) {
        return NULL;
    }

    for (size_t i = 0; i <= haystack_len - needle_len; ++i) {
        if (memcmp(haystack + i, needle, needle_len) == 0) {
            return haystack + i;
        }
    }

    return NULL;
}

typedef struct {
    char field_name[32];
    char filename[64];
    char content_type[64];
} web_server_multipart_headers_t;

static esp_err_t web_server_extract_boundary(const char *content_type,
                                             char *boundary,
                                             size_t boundary_size)
{
    if (content_type == NULL || boundary == NULL || boundary_size < 4U) {
        return ESP_ERR_INVALID_ARG;
    }

    if (strstr(content_type, "multipart/form-data") == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    const char *needle = "boundary=";
    const char *position = strstr(content_type, needle);
    if (position == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    position += strlen(needle);
    if (*position == '\"') {
        ++position;
    }

    const char *end = position;
    while (*end != '\0' && *end != ';' && *end != ' ' && *end != '\"') {
        ++end;
    }

    size_t boundary_value_len = (size_t)(end - position);
    if (boundary_value_len == 0 || boundary_value_len + 2U >= boundary_size) {
        return ESP_ERR_INVALID_SIZE;
    }

    int written = snprintf(boundary, boundary_size, "--%.*s", (int)boundary_value_len, position);
    if (written < 0 || (size_t)written >= boundary_size) {
        return ESP_ERR_INVALID_SIZE;
    }

    return ESP_OK;
}

static ssize_t web_server_parse_multipart_headers(uint8_t *buffer,
                                                  size_t buffer_len,
                                                  const char *boundary_line,
                                                  web_server_multipart_headers_t *out_headers)
{
    if (buffer == NULL || boundary_line == NULL || out_headers == NULL) {
        return -2;
    }

    const size_t boundary_len = strlen(boundary_line);
    if (buffer_len < boundary_len + 2U) {
        return -1;
    }

    if (memcmp(buffer, boundary_line, boundary_len) != 0) {
        return -2;
    }

    const uint8_t *cursor = buffer + boundary_len;
    const uint8_t *buffer_end = buffer + buffer_len;

    if (cursor + 2 > buffer_end || cursor[0] != '\r' || cursor[1] != '\n') {
        return -1;
    }
    cursor += 2;

    bool has_disposition = false;
    memset(out_headers, 0, sizeof(*out_headers));

    while (cursor < buffer_end) {
        const uint8_t *line_end = web_server_memmem(cursor, (size_t)(buffer_end - cursor), (const uint8_t *)"\r\n", 2);
        if (line_end == NULL) {
            return -1;
        }

        size_t line_length = (size_t)(line_end - cursor);
        if (line_length == 0) {
            cursor = line_end + 2;
            break;
        }

        if (line_length >= WEB_SERVER_MULTIPART_HEADER_MAX) {
            return -2;
        }

        char line[WEB_SERVER_MULTIPART_HEADER_MAX];
        memcpy(line, cursor, line_length);
        line[line_length] = '\0';

        if (strncasecmp(line, "Content-Disposition:", 20) == 0) {
            const char *name_token = strstr(line, "name=");
            if (name_token != NULL) {
                name_token += 5;
                if (*name_token == '\"') {
                    ++name_token;
                    const char *name_end = strchr(name_token, '\"');
                    if (name_end != NULL) {
                        size_t name_len = (size_t)(name_end - name_token);
                        if (name_len < sizeof(out_headers->field_name)) {
                            memcpy(out_headers->field_name, name_token, name_len);
                            out_headers->field_name[name_len] = '\0';
                        }
                    }
                }
            }

            const char *filename_token = strstr(line, "filename=");
            if (filename_token != NULL) {
                filename_token += 9;
                if (*filename_token == '\"') {
                    ++filename_token;
                    const char *filename_end = strchr(filename_token, '\"');
                    if (filename_end != NULL) {
                        size_t filename_len = (size_t)(filename_end - filename_token);
                        if (filename_len < sizeof(out_headers->filename)) {
                            memcpy(out_headers->filename, filename_token, filename_len);
                            out_headers->filename[filename_len] = '\0';
                        }
                    }
                }
            }

            has_disposition = true;
        } else if (strncasecmp(line, "Content-Type:", 13) == 0) {
            const char *value = line + 13;
            while (*value == ' ' || *value == '\t') {
                ++value;
            }
            size_t len = strnlen(value, sizeof(out_headers->content_type) - 1U);
            memcpy(out_headers->content_type, value, len);
            out_headers->content_type[len] = '\0';
        }

        cursor = line_end + 2;
    }

    if (!has_disposition) {
        return -2;
    }

    return (ssize_t)(cursor - buffer);
}

static esp_err_t web_server_process_multipart_body(uint8_t *buffer,
                                                   size_t *buffer_len,
                                                   const char *boundary_marker,
                                                   ota_update_session_t *session,
                                                   size_t *total_written,
                                                   bool *complete)
{
    if (buffer == NULL || buffer_len == NULL || boundary_marker == NULL || session == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    const size_t marker_len = strlen(boundary_marker);
    const size_t guard = marker_len + 8U;
    size_t processed = 0;

    while (processed < *buffer_len) {
        size_t available = *buffer_len - processed;
        if (available == 0) {
            break;
        }

        const uint8_t *marker = web_server_memmem(buffer + processed, available,
                                                  (const uint8_t *)boundary_marker, marker_len);
        if (marker == NULL) {
            if (available <= guard) {
                break;
            }

            size_t chunk = available - guard;
            if (chunk > 0) {
                esp_err_t err = ota_update_write(session, buffer + processed, chunk);
                if (err != ESP_OK) {
                    return err;
                }
                if (total_written != NULL) {
                    *total_written += chunk;
                }
                processed += chunk;
                continue;
            }
            break;
        }

        size_t marker_index = (size_t)(marker - buffer);
        if (marker_index > processed) {
            size_t chunk = marker_index - processed;
            esp_err_t err = ota_update_write(session, buffer + processed, chunk);
            if (err != ESP_OK) {
                return err;
            }
            if (total_written != NULL) {
                *total_written += chunk;
            }
        }

        size_t after_marker = marker_index + marker_len;
        bool final = false;
        if (*buffer_len - after_marker >= 2 && memcmp(buffer + after_marker, "--", 2) == 0) {
            final = true;
            after_marker += 2;
        }
        if (*buffer_len - after_marker >= 2 && memcmp(buffer + after_marker, "\r\n", 2) == 0) {
            after_marker += 2;
        }

        processed = after_marker;
        if (complete != NULL) {
            *complete = final;
        }

        if (!final) {
            return ESP_ERR_INVALID_RESPONSE;
        }

        break;
    }

    if (processed > 0) {
        size_t remaining = *buffer_len - processed;
        if (remaining > 0) {
            memmove(buffer, buffer + processed, remaining);
        }
        *buffer_len = remaining;
    }

    return ESP_OK;
}

static esp_err_t web_server_stream_firmware_upload(httpd_req_t *req,
                                                   ota_update_session_t *session,
                                                   const char *boundary_line,
                                                   web_server_multipart_headers_t *headers,
                                                   size_t *out_written)
{
    if (req == NULL || session == NULL || boundary_line == NULL || headers == NULL) {
        return ESP_ERR_INVALID_ARG;
    }

    uint8_t buffer[WEB_SERVER_MULTIPART_BUFFER_SIZE];
    size_t buffer_len = 0U;
    size_t received = 0U;
    bool headers_parsed = false;
    bool upload_complete = false;
    size_t total_written = 0U;

    char boundary_marker[WEB_SERVER_MULTIPART_BOUNDARY_MAX + 4];
    int marker_written = snprintf(boundary_marker,
                                  sizeof(boundary_marker),
                                  "\r\n%s",
                                  boundary_line);
    if (marker_written < 0 || (size_t)marker_written >= sizeof(boundary_marker)) {
        return ESP_ERR_INVALID_SIZE;
    }

    while (!upload_complete || buffer_len > 0U || received < (size_t)req->content_len) {
        if (received < (size_t)req->content_len) {
            if (buffer_len >= sizeof(buffer)) {
                return ESP_ERR_INVALID_SIZE;
            }
            size_t to_read = sizeof(buffer) - buffer_len;
            int ret = httpd_req_recv(req, (char *)buffer + buffer_len, to_read);
            if (ret < 0) {
                if (ret == HTTPD_SOCK_ERR_TIMEOUT) {
                    continue;
                }
                return ESP_FAIL;
            }
            if (ret == 0) {
                break;
            }
            buffer_len += (size_t)ret;
            received += (size_t)ret;
        }

        if (!headers_parsed) {
            ssize_t header_end = web_server_parse_multipart_headers(buffer, buffer_len, boundary_line, headers);
            if (header_end == -1) {
                continue;
            }
            if (header_end < 0) {
                return ESP_ERR_INVALID_RESPONSE;
            }

            size_t data_len = buffer_len - (size_t)header_end;
            if (data_len > 0) {
                memmove(buffer, buffer + header_end, data_len);
            }
            buffer_len = data_len;
            headers_parsed = true;
        }

        if (headers_parsed) {
            esp_err_t err = web_server_process_multipart_body(buffer,
                                                              &buffer_len,
                                                              boundary_marker,
                                                              session,
                                                              &total_written,
                                                              &upload_complete);
            if (err == ESP_ERR_INVALID_RESPONSE) {
                return err;
            }
            if (err != ESP_OK) {
                return err;
            }
        }

        if (upload_complete && buffer_len == 0U && received >= (size_t)req->content_len) {
            break;
        }
    }

    if (!upload_complete) {
        return ESP_ERR_INVALID_RESPONSE;
    }

    if (out_written != NULL) {
        *out_written = total_written;
    }

    return ESP_OK;
}

esp_err_t web_server_api_ota_post_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, true, NULL, 0)) {
        return ESP_FAIL;
    }

    if (req->content_len == 0) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_EMPTY_PAYLOAD, NULL, NULL);
    }

    char content_type[WEB_SERVER_MULTIPART_HEADER_MAX];
    if (httpd_req_get_hdr_value_str(req, "Content-Type", content_type, sizeof(content_type)) != ESP_OK) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_MISSING_CONTENT_TYPE, NULL, NULL);
    }

    char boundary_line[WEB_SERVER_MULTIPART_BOUNDARY_MAX];
    esp_err_t err = web_server_extract_boundary(content_type, boundary_line, sizeof(boundary_line));
    if (err != ESP_OK) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_INVALID_BOUNDARY, NULL, NULL);
    }

    ota_update_session_t *session = NULL;
    err = ota_update_begin(&session, req->content_len);
    if (err != ESP_OK) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_SUBSYSTEM_BUSY, NULL, NULL);
    }

    web_server_multipart_headers_t headers;
    size_t bytes_written = 0U;
    err = web_server_stream_firmware_upload(req, session, boundary_line, &headers, &bytes_written);
    if (err != ESP_OK) {
        ota_update_abort(session);
        web_server_ota_error_code_t code = (err == ESP_ERR_INVALID_RESPONSE)
            ? WEB_SERVER_OTA_ERROR_MALFORMED_MULTIPART
            : WEB_SERVER_OTA_ERROR_STREAM_FAILURE;
        return web_server_send_ota_response(req, code, NULL, NULL);
    }

    (void)bytes_written;

    if (headers.field_name[0] == '\0' || strcmp(headers.field_name, "firmware") != 0) {
        ota_update_abort(session);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_MISSING_FIRMWARE_FIELD, NULL, NULL);
    }

    if (headers.content_type[0] != '\0' &&
        strncasecmp(headers.content_type, "application/octet-stream", sizeof(headers.content_type)) != 0 &&
        strncasecmp(headers.content_type, "application/x-binary", sizeof(headers.content_type)) != 0) {
        ota_update_abort(session);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_UNSUPPORTED_CONTENT_TYPE, NULL, NULL);
    }

    ota_update_result_t result = {0};
    err = ota_update_finalize(session, &result);
    if (err != ESP_OK) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_VALIDATION_FAILED, NULL, NULL);
    }

    if (s_event_publisher != NULL) {
        const char *filename = (headers.filename[0] != '\0') ? headers.filename : "firmware.bin";
        int label_written = snprintf(s_ota_event_label,
                                     sizeof(s_ota_event_label),
                                     "%s (%zu bytes, crc32=%08" PRIX32 " )",
                                     filename,
                                     result.bytes_written,
                                     result.crc32);
        if (label_written > 0 && (size_t)label_written < sizeof(s_ota_event_label)) {
#ifdef ESP_PLATFORM
            s_ota_event_metadata.timestamp_ms = (uint64_t)(esp_timer_get_time() / 1000ULL);
#else
            s_ota_event_metadata.timestamp_ms = 0U;
#endif
            event_bus_event_t event = {
                .id = APP_EVENT_ID_OTA_UPLOAD_READY,
                .payload = &s_ota_event_metadata,
                .payload_size = sizeof(s_ota_event_metadata),
            };
            s_event_publisher(&event, pdMS_TO_TICKS(50));
        }
    }

    cJSON *data = cJSON_CreateObject();
    if (data == NULL) {
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    if (cJSON_AddNumberToObject(data, "bytes", (double)result.bytes_written) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    char crc_buffer[9];
    snprintf(crc_buffer, sizeof(crc_buffer), "%08" PRIX32, result.crc32);
    if (cJSON_AddStringToObject(data, "crc32", crc_buffer) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    const char *partition = (result.partition_label[0] != '\0') ? result.partition_label : "unknown";
    if (cJSON_AddStringToObject(data, "partition", partition) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    const char *version = (result.new_version[0] != '\0') ? result.new_version : "unknown";
    if (cJSON_AddStringToObject(data, "version", version) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    if (cJSON_AddBoolToObject(data, "reboot_required", result.reboot_required) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    if (cJSON_AddBoolToObject(data, "version_changed", result.version_changed) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    const char *filename = (headers.filename[0] != '\0') ? headers.filename : "firmware.bin";
    if (cJSON_AddStringToObject(data, "filename", filename) == NULL) {
        cJSON_Delete(data);
        return web_server_send_ota_response(req, WEB_SERVER_OTA_ERROR_ENCODING_FAILED, NULL, NULL);
    }

    return web_server_send_ota_response(req, WEB_SERVER_OTA_OK, NULL, data);
}

esp_err_t web_server_api_restart_post_handler(httpd_req_t *req)
{
    if (!web_server_require_authorization(req, true, NULL, 0)) {
        return ESP_FAIL;
    }

    char body[256] = {0};
    size_t received = 0U;

    if ((size_t)req->content_len >= sizeof(body)) {
        httpd_resp_send_err(req, HTTPD_413_PAYLOAD_TOO_LARGE, "Restart payload too large");
        return ESP_ERR_INVALID_SIZE;
    }

    while (received < (size_t)req->content_len) {
        int ret = httpd_req_recv(req, body + received, req->content_len - received);
        if (ret < 0) {
            if (ret == HTTPD_SOCK_ERR_TIMEOUT) {
                continue;
            }
            httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Failed to read restart payload");
            return ESP_FAIL;
        }
        if (ret == 0) {
            break;
        }
        received += (size_t)ret;
    }

    char target_buf[16] = "bms";
    const char *target = target_buf;
    uint32_t delay_ms = WEB_SERVER_RESTART_DEFAULT_DELAY_MS;

    if (received > 0U) {
        body[received] = '\0';
        cJSON *json = cJSON_Parse(body);
        if (json == NULL) {
            httpd_resp_send_err(req, HTTPD_400_BAD_REQUEST, "Invalid JSON payload");
            return ESP_ERR_INVALID_ARG;
        }

        const cJSON *target_item = cJSON_GetObjectItemCaseSensitive(json, "target");
        if (cJSON_IsString(target_item) && target_item->valuestring != NULL) {
            strncpy(target_buf, target_item->valuestring, sizeof(target_buf) - 1U);
            target_buf[sizeof(target_buf) - 1U] = '\0';
        }

        const cJSON *delay_item = cJSON_GetObjectItemCaseSensitive(json, "delay_ms");
        if (cJSON_IsNumber(delay_item) && delay_item->valuedouble >= 0.0) {
            delay_ms = (uint32_t)delay_item->valuedouble;
        }

        cJSON_Delete(json);
    }

    bool request_gateway_restart = false;
    bool bms_attempted = false;
    const char *bms_status = "skipped";
    esp_err_t bms_err = ESP_OK;

    if (target != NULL && strcasecmp(target, "gateway") == 0) {
        request_gateway_restart = true;
    } else {
        bms_attempted = true;
        bms_err = system_control_request_bms_restart(0U);
        if (bms_err == ESP_OK) {
            bms_status = "ok";
        } else if (bms_err == ESP_ERR_INVALID_STATE) {
            bms_status = "throttled";
        } else if (bms_err == ESP_ERR_TIMEOUT) {
            bms_status = "timeout";
        } else {
            bms_status = esp_err_to_name(bms_err);
        }

        if (bms_err != ESP_OK) {
            request_gateway_restart = true;
        }
    }

    if (request_gateway_restart) {
        esp_err_t gw_err = system_control_schedule_gateway_restart(delay_ms);
        if (gw_err != ESP_OK) {
            httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Failed to schedule gateway restart");
            return gw_err;
        }
    }

    if (s_event_publisher != NULL) {
        const char *mode = request_gateway_restart ? "gateway" : "bms";
        const char *suffix = (request_gateway_restart && bms_attempted && bms_err != ESP_OK) ? "+fallback" : "";
        int label_written = snprintf(s_restart_event_label,
                                     sizeof(s_restart_event_label),
                                     "Restart requested (%s%s)",
                                     mode,
                                     suffix);
        if (label_written > 0 && (size_t)label_written < sizeof(s_restart_event_label)) {
#ifdef ESP_PLATFORM
            s_restart_event_metadata.timestamp_ms = (uint64_t)(esp_timer_get_time() / 1000ULL);
#else
            s_restart_event_metadata.timestamp_ms = 0U;
#endif
            event_bus_event_t event = {
                .id = APP_EVENT_ID_UI_NOTIFICATION,
                .payload = &s_restart_event_metadata,
                .payload_size = sizeof(s_restart_event_metadata),
            };
            s_event_publisher(&event, pdMS_TO_TICKS(50));
        }
    }

    char response[256];
    int written = snprintf(response,
                           sizeof(response),
                           "{\"status\":\"scheduled\",\"bms_attempted\":%s,\"bms_status\":\"%s\",\"gateway_restart\":%s,\"delay_ms\":%u}",
                           bms_attempted ? "true" : "false",
                           bms_status,
                           request_gateway_restart ? "true" : "false",
                           request_gateway_restart ? delay_ms : 0U);
    if (written < 0 || (size_t)written >= sizeof(response)) {
        httpd_resp_send_err(req, HTTPD_500_INTERNAL_SERVER_ERROR, "Restart response too large");
        return ESP_ERR_INVALID_SIZE;
    }

    if (request_gateway_restart) {
        httpd_resp_set_status(req, "202 Accepted");
    }

    return web_server_send_json(req, response, (size_t)written);
}
