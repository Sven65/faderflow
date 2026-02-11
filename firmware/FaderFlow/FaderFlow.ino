#include "protocol.h"
#include "device_id.h"
#include "comms.h"
#include "fader.h"

static bool handshakeComplete = false;

void setup() {
    Serial.begin(115200);
    initDeviceID();
    initFaders();

    // Wait for serial to stabilize
    delay(1000);
}

void loop() {
    // Handle incoming commands
    if (Serial.available() > 0) {
        uint8_t cmd = Serial.read();

        if (cmd == CMD_HANDSHAKE_REQUEST) {
            sendHandshake();
            handshakeComplete = true;  // Enable fader sending
        }
        else if (cmd == CMD_ECHO_UUID) {
            uint8_t uuid[UUID_SIZE];
            getDeviceUUID(uuid);
            Serial.write(uuid, UUID_SIZE);
        }
        else if (cmd == 'h') {
            sendHandshake();
            handshakeComplete = true;
        }
        else if (cmd == 'u') {
            uint8_t uuid[UUID_SIZE];
            getDeviceUUID(uuid);
            Serial.write(uuid, UUID_SIZE);
        }
    }

    // Only read faders after handshake
    if (handshakeComplete) {
        readFaders();
    }
}