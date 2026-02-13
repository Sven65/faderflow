#include "src/protocol.h"
#include "src/utils/device_id.h"
#include "src/utils/comms.h"
#include "src/components/fader.h"
#include <SPI.h>
#include "src/channel.h"

// Create ONE channel for testing
Channel testChannel(0, 3, 2, 8, 1, 1, 1);
static bool handshakeComplete = false;

void setup() {
    pinMode(53, OUTPUT);
    digitalWrite(53, LOW);
    Serial.begin(115200);
    Serial.setTimeout(50);
    initDeviceID();
    initFaders();
    
    // Initialize SPI and display
    SPI.begin();
    testChannel.begin();
    testChannel.getIcon()->useTestIcon();
    testChannel.setApp("Test App");
    testChannel.updateIconDisplay();
    
    delay(1000);
}

void loop() {
    static unsigned long lastBeacon = 0;
    
    // Auto-announce until handshake complete
    if (!handshakeComplete) {
        if (millis() - lastBeacon > 500) {
            sendHandshake();
            lastBeacon = millis();
        }
    }
    
    if (Serial.available() > 0) {
        uint8_t cmd = Serial.read();
        
        if (cmd == CMD_HANDSHAKE_REQUEST || cmd == 'h') {
            sendHandshake();
            handshakeComplete = true;
        }
        else if (cmd == CMD_HANDSHAKE_ACK) {
            handshakeComplete = true;  // Stop beaconing
        }
        else if (cmd == CMD_ECHO_UUID || cmd == 'u') {
            uint8_t uuid[UUID_SIZE];
            getDeviceUUID(uuid);
            Serial.write(uuid, UUID_SIZE);
        }
        else if (cmd == CMD_DISPLAY_UPDATE_APP_NAME) {
            handleAppNameUpdate();
        }
        else if (cmd == CMD_DISPLAY_UPDATE_APP_VOLUME) {
            handleVolumeUpdate();
        }
    }
    
    //testChannel.update(); // Commented out - encoder not connected
    
    if (handshakeComplete) {
        //readFaders();
    }
}

void handleAppNameUpdate() {
    DisplayUpdateAppCommand cmd;
    size_t bytesToRead = sizeof(DisplayUpdateAppCommand) - 1;
    
    if (Serial.readBytes((uint8_t*)&cmd.channel, bytesToRead) != bytesToRead) {
        return;
    }
    
    cmd.name[63] = '\0';  // Force null termination
    
    testChannel.setApp(cmd.name);
}

void handleVolumeUpdate() {
    DisplayUpdateVolumeCommand cmd;
    size_t bytesToRead = sizeof(DisplayUpdateVolumeCommand) - 1;
    
    if (Serial.readBytes((uint8_t*)&cmd.channel, bytesToRead) != bytesToRead) {
        return;
    }
    
    testChannel.setVolume(cmd.volume);
}