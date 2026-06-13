//
// Created by Mackan on 2026-02-13.
// Updated 2026-06-12: Fader component integrated.
//

#ifndef FADERFLOW_CHANNEL_H
#define FADERFLOW_CHANNEL_H

#include <Arduino.h>
#include "components/display.h"
#include "components/RotaryEncoder.h"
#include "components/fader.h"
#include "components/icon.h"

class Channel {
public:
  Channel(
    uint8_t id,
    // Display pins
    int8_t displayCS, int8_t displayDC, int8_t displayRST,
    // Encoder pins
    uint8_t encoderDT, uint8_t encoderCLK, uint8_t encoderSW,
    // Fader pins
    uint8_t faderMotorA, uint8_t faderMotorB, uint8_t faderAnalog
  );

  // Initialize all components
  void begin();

  // Update all components (call in loop)
  void update();

  // Set the current app
  void setApp(const char* appName);

  // Set the volume (from host) — updates display AND moves the fader
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

  // Check if the user moved the fader
  bool hasFaderChanged();

  // Check if button was pressed
  bool wasButtonPressed();

  bool receiveIcon(Stream& s);
  void stopFader();
  void releaseFader();  // coast (free) instead of brake

  // ---- Calibration support ----
  int faderRaw();
  void setFaderCalibration(int rawMin, int rawMax);
  void showMessage(const char* l1, const char* l2, const char* l3);
  void redrawUI();
  void pollEncoderButton();   // encoder only — no display/fader side effects
  void flushInputs();         // discard accumulated encoder/fader events

private:
  uint8_t id;
  Display display;
  RotaryEncoder encoder;
  Fader fader;
  Icon icon;

  String appName;
  int volume;
  bool encoderChanged;
  int encoderDelta;
  bool faderChanged;

  bool displayDirty;
  uint32_t lastDisplayDraw;
};

#endif //FADERFLOW_CHANNEL_H