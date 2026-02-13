//
// Created by Mackan on 2026-02-13.
//

#ifndef FADERFLOW_ROTARYENCODER_H
#define FADERFLOW_ROTARYENCODER_H

#include <Arduino.h>

class RotaryEncoder {
public:
    RotaryEncoder(uint8_t pinDT, uint8_t pinCLK, uint8_t pinSW);

    // Initialize the encoder
    void begin();

    // Update encoder state (call in loop)
    void update();

    // Get the current position delta since last read
    int getDelta();

    // Check if button was pressed
    bool wasPressed();

    // Reset the encoder state
    void reset();

private:
    uint8_t pinDT;
    uint8_t pinCLK;
    uint8_t pinSW;

    // Encoder state
    int position;
    int lastPosition;
    uint8_t lastStateCLK;

    // Button state
    bool buttonState;
    bool lastButtonState;
    unsigned long lastDebounceTime;
    bool buttonPressed;

    static const unsigned long DEBOUNCE_DELAY = 50;
};

#endif //FADERFLOW_ROTARYENCODER_H