//
// Created by Mackan on 2026-02-11.
// Reworked 2026-06-12: class-based, motor control merged in, protocol decoupled.
//

#ifndef FADERFLOW_FADER_H
#define FADERFLOW_FADER_H

#include <Arduino.h>

#define FADER_DEADBAND 2          // % change before a move event fires
#define FADER_READ_INTERVAL 15    // ms between reads
#define FADER_INVERTED 1          // wiper reads 1023 at bottom (confirmed in bring-up)

#define FADER_SEEK_DEADBAND 2     // ±% considered "arrived"
#define FADER_SEEK_MIN_SPEED 110  // PWM floor to overcome static friction
#define FADER_SEEK_TIMEOUT 1500   // ms safety stop

class Fader {
public:
    Fader(uint8_t motorA, uint8_t motorB, uint8_t analogPin);

    void begin();

    // Call every loop. Runs the seek loop if a target is set,
    // otherwise watches for the user moving the fader by hand.
    void update();

    // Motor-drive the fader to a position (0-100). Non-blocking.
    // Touch events are suppressed while seeking.
    void setTarget(int percent);

    // Immediate raw position read, 0-100 (8-sample average)
    int read();

    // Smoothed current position, 0-100
    int getPosition();

    bool isSeeking();

    // True once if the user moved the fader (cleared on call)
    bool hasMoved();

    void stop();

private:
    uint8_t motorA;
    uint8_t motorB;
    uint8_t analogPin;

    int lastRawValue;          // EMA accumulator (raw 0-1023)
    int lastReported;          // last position a move event fired for (0-100)
    unsigned long lastReadTime;

    int target;                // -1 = idle
    bool seeking;
    unsigned long seekStart;

    bool moved;

    // speed: -255..255. Positive = up (motorA PWM).
    // Zero = brake (both high), stops the fader dead instead of coasting.
    void motorWrite(int speed);

    int rawToPercent(int raw);
};

#endif //FADERFLOW_FADER_H