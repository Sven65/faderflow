//
// Created by Mackan on 2026-02-13.
//

#include "RotaryEncoder.h"

RotaryEncoder::RotaryEncoder(uint8_t pinDT, uint8_t pinCLK, uint8_t pinSW) {
    this->pinDT = pinDT;
    this->pinCLK = pinCLK;
    this->pinSW = pinSW;

    position = 0;
    lastPosition = 0;
    lastStateCLK = LOW;
    buttonState = HIGH;
    lastButtonState = HIGH;
    lastDebounceTime = 0;
    buttonPressed = false;
}

void RotaryEncoder::begin() {
    pinMode(pinDT, INPUT_PULLUP);
    pinMode(pinCLK, INPUT_PULLUP);
    pinMode(pinSW, INPUT_PULLUP);

    lastStateCLK = digitalRead(pinCLK);
}

void RotaryEncoder::update() {
    // Read encoder rotation
    uint8_t currentStateCLK = digitalRead(pinCLK);

    if (currentStateCLK != lastStateCLK && currentStateCLK == HIGH) {
        // CLK changed from LOW to HIGH (detent position)
        if (digitalRead(pinDT) != currentStateCLK) {
            position++; // Clockwise
        } else {
            position--; // Counter-clockwise
        }
    }

    lastStateCLK = currentStateCLK;

    // Read button with debouncing
    bool reading = digitalRead(pinSW);

    if (reading != lastButtonState) {
        lastDebounceTime = millis();
    }

    if ((millis() - lastDebounceTime) > DEBOUNCE_DELAY) {
        if (reading != buttonState) {
            buttonState = reading;

            // Button was pressed (LOW = pressed)
            if (buttonState == LOW) {
                buttonPressed = true;
            }
        }
    }

    lastButtonState = reading;
}

int RotaryEncoder::getDelta() {
    int delta = position - lastPosition;
    lastPosition = position;
    return delta;
}

bool RotaryEncoder::wasPressed() {
    bool pressed = buttonPressed;
    buttonPressed = false; // Clear the flag
    return pressed;
}

void RotaryEncoder::reset() {
    position = 0;
    lastPosition = 0;
    buttonPressed = false;
}