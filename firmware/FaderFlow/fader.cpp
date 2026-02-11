#include "fader.h"
#include "protocol.h"

const uint8_t faderPins[NUM_FADERS] = {
    FADER_1_PIN,
    FADER_2_PIN,
    FADER_3_PIN,
    FADER_4_PIN,
    FADER_5_PIN
};

struct FaderState {
    int lastRawValue;
    uint8_t lastSentValue;
    unsigned long lastReadTime;
};

static FaderState faderStates[NUM_CONNECTED_FADERS];

void initFaders() {
    for (int i = 0; i < NUM_CONNECTED_FADERS; i++) {
        pinMode(faderPins[i], INPUT);
        faderStates[i].lastRawValue = -1;
        faderStates[i].lastSentValue = 255;
        faderStates[i].lastReadTime = 0;
    }
}

void readFaders() {
    unsigned long now = millis();

    for (uint8_t i = 0; i < NUM_CONNECTED_FADERS; i++) {
        FaderState* state = &faderStates[i];

        // Rate limit per fader
        if (now - state->lastReadTime < FADER_READ_INTERVAL) {
            continue;
        }
        state->lastReadTime = now;

        analogRead(faderPins[i]);
        delayMicroseconds(100);
        // Read current position
        int rawValue = analogRead(faderPins[i]);

        // Simple moving average for smoothing
        if (state->lastRawValue == -1) {
            state->lastRawValue = rawValue;
        } else {
            state->lastRawValue = (state->lastRawValue * 3 + rawValue) / 4;
        }

        // Map to 0-255
        uint8_t faderPos = map(state->lastRawValue, 0, 1023, 0, 255);

        // Only send if changed beyond deadband
        if (abs(faderPos - state->lastSentValue) > FADER_DEADBAND) {
            state->lastSentValue = faderPos;

            // Send binary message
            FaderMessage msg;
            msg.cmd = CMD_FADER_UPDATE;
            msg.channel = i;
            msg.position = faderPos;

            Serial.write((uint8_t*)&msg, sizeof(msg));
        }
    }
}