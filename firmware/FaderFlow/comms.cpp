//
// Created by Mackan on 2026-02-11.
//

#include "comms.h"
#include "protocol.h"
#include "device_id.h"
#include <string.h>

void sendHandshake() {
    HandshakeResponse response;
    strcpy(response.magic, MAGIC_STRING);
    response.device_type = 0x01;
    getDeviceUUID(response.uuid);
    response.version_major = 1;
    response.version_minor = 0;

    Serial.write((uint8_t*)&response, sizeof(response));
}