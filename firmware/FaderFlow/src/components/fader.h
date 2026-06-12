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
#define FADER_RAW_MIN 12          // raw ADC at one physical end stop (pot dead zone)
#define FADER_RAW_MAX 1011        // raw ADC at the other end stop

#define FADER_SEEK_DEADBAND 2     // ±% considered "arrived"
#define FADER_SEEK_MIN_SPEED 110  // PWM floor to overcome static friction
#define FADER_SEEK_SLOW_ZONE 6    // % from target where speed ramps down
#define FADER_SEEK_CRAWL 85       // creep PWM (dynamic friction < static)
#define FADER_SEEK_SETTLE_MS 60   // post-brake rest before verifying position
#define FADER_SEEK_MAX_RETRIES 2  // bounce-correction creep attempts
#define FADER_STALL_SAMPLES 3     // stalled-at-max-boost samples = against end stop
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

    // Per-unit calibration: raw ADC values at the physical end stops.
    // Overrides the FADER_RAW_MIN/MAX compile-time defaults.
    void setCalibration(int rawMin, int rawMax);

    // 8-sample averaged raw ADC reading (used during calibration capture)
    int readRawAveraged();

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
    int prevSeekPos;           // for velocity estimate
    unsigned long lastVelSample;
    int8_t velocity;           // %/sample, signed
    unsigned long settleUntil; // 0 = not settling
    uint8_t seekRetries;
    uint8_t crawlBoost;        // anti-stall PWM bump during creep
    uint8_t stallCount;        // consecutive stalled samples at max boost
    int calMin;                // raw ADC at 0% end stop side
    int calMax;                // raw ADC at 100% end stop side

    bool moved;

    // speed: -255..255. Positive = up (motorA PWM).
    // Zero = brake (both high), stops the fader dead instead of coasting.
    void motorWrite(int speed);

    int rawToPercent(int raw);
};

#endif //FADERFLOW_FADER_H