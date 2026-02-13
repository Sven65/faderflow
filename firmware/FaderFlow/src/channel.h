//
// Created by Mackan on 2026-02-13.
//

#ifndef FADERFLOW_CHANNEL_H
#define FADERFLOW_CHANNEL_H

#include <Arduino.h>
#include "components/display.h"
#include "components/RotaryEncoder.h"
#include "components/icon.h"

class Channel {
  public:
    // Constructor
    Channel(
      uint8_t id,
      // Display pins
      int8_t displayCS, int8_t displayDC, int8_t displayRST,
      // Encoder pins
      uint8_t encoderDT, uint8_t encoderCLK, uint8_t encoderSW
      // Fader pins (for later)
      // uint8_t faderPWM1, uint8_t faderPWM2, uint8_t faderAnalog
    );

    // Initialize all components
    void begin();

    // Update all components (call in loop)
    void update();

    // Set the current app
    void setApp(const char* appName);

    // Set the volume
    void setVolume(int volume);

    // Get current volume
    int getVolume();

    // Get channel ID
    uint8_t getID();

    // Icon management
    Icon* getIcon();
    void updateIconDisplay();

    // Check if encoder changed
    bool hasEncoderChanged();
    int getEncoderDelta();

    // Check if button was pressed
    bool wasButtonPressed();

  private:
    uint8_t id;
    Display display;
    RotaryEncoder encoder;
    Icon icon;

    String appName;
    int volume;
    bool encoderChanged;
    int encoderDelta;
};

#endif //FADERFLOW_CHANNEL_H