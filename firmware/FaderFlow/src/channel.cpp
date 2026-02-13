//
// Created by Mackan on 2026-02-13.
//

#include "channel.h"

Channel::Channel(
  uint8_t id,
  int8_t displayCS, int8_t displayDC, int8_t displayRST,
  uint8_t encoderDT, uint8_t encoderCLK, uint8_t encoderSW
) : id(id),
    display(displayCS, displayDC, displayRST),
    encoder(encoderDT, encoderCLK, encoderSW) {

  appName = "Waiting...";
  volume = 50;
  encoderChanged = false;
  encoderDelta = 0;
}

void Channel::begin() {
  display.begin();
  encoder.begin();
  display.drawUI(volume, appName.c_str(), &icon);
}

void Channel::update() {
  encoder.update();

  // Check for encoder changes
  int delta = encoder.getDelta();
  if (delta != 0) {
    encoderChanged = true;
    encoderDelta = delta;

    // Update volume locally
    volume += delta;
    volume = constrain(volume, 0, 100);

    // Update display
    display.updateVolume(volume);
  }
}

void Channel::setApp(const char* appName) {
  this->appName = String(appName);
  display.updateAppName(this->appName.c_str());
}

void Channel::setVolume(int volume) {
  this->volume = constrain(volume, 0, 100);
  display.updateVolume(this->volume);
}

int Channel::getVolume() {
  return volume;
}

uint8_t Channel::getID() {
  return id;
}

Icon* Channel::getIcon() {
  return &icon;
}

void Channel::updateIconDisplay() {
  display.updateIcon(&icon);
}

bool Channel::hasEncoderChanged() {
  bool changed = encoderChanged;
  encoderChanged = false;
  return changed;
}

int Channel::getEncoderDelta() {
  int delta = encoderDelta;
  encoderDelta = 0;
  return delta;
}

bool Channel::wasButtonPressed() {
  return encoder.wasPressed();
}