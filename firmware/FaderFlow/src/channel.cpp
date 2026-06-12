//
// Created by Mackan on 2026-02-13.
// Updated 2026-06-12: Fader component integrated.
//

#include "channel.h"

Channel::Channel(
  uint8_t id,
  int8_t displayCS, int8_t displayDC, int8_t displayRST,
  uint8_t encoderDT, uint8_t encoderCLK, uint8_t encoderSW,
  uint8_t faderMotorA, uint8_t faderMotorB, uint8_t faderAnalog
) : id(id),
    display(displayCS, displayDC, displayRST),
    encoder(encoderDT, encoderCLK, encoderSW),
    fader(faderMotorA, faderMotorB, faderAnalog) {

  appName = "Waiting...";
  volume = 50;
  encoderChanged = false;
  encoderDelta = 0;
  faderChanged = false;
  displayDirty = false;
  lastDisplayDraw = 0;
}

void Channel::begin() {
  display.begin();
  encoder.begin();
  fader.begin();

  // Adopt the fader's physical position as the starting volume
  volume = fader.getPosition();

  display.drawUI(volume, appName.c_str(), &icon);
}

void Channel::update() {
  encoder.update();
  fader.update();

  int delta = encoder.getDelta();
  if (delta != 0) {
    encoderChanged = true;
    encoderDelta += delta;
    volume = constrain(volume + delta, 0, 100);
    fader.setTarget(volume);
    displayDirty = true;          // ← was: display.updateVolume(volume);
  }

  if (fader.hasMoved()) {
    volume = fader.getPosition();
    faderChanged = true;
    displayDirty = true;          // ← was: display.updateVolume(volume);
  }

  // Throttled redraw — screen at ~12Hz, everything else at full speed
  if (displayDirty && millis() - lastDisplayDraw >= 80) {
    display.updateVolume(volume);
    lastDisplayDraw = millis();
    displayDirty = false;
  }
}

void Channel::setApp(const char* appName) {
  this->appName = String(appName);
  display.updateAppName(this->appName.c_str());
}

void Channel::setVolume(int volume) {
  this->volume = constrain(volume, 0, 100);
  display.updateVolume(this->volume);
  fader.setTarget(this->volume);
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

bool Channel::hasFaderChanged() {
  bool changed = faderChanged;
  faderChanged = false;
  return changed;
}

bool Channel::wasButtonPressed() {
  return encoder.wasPressed();
}
bool Channel::receiveIcon(Stream& s) {
  display.beginIconStream();

  const uint16_t total = (uint16_t)ICON_SIZE * ICON_SIZE;
  uint16_t pixelsDone = 0;
  uint8_t hi = 0;
  bool haveHigh = false;
  uint32_t lastByte = millis();

  while (pixelsDone < total) {
    if (s.available()) {
      uint8_t b = s.read();
      lastByte = millis();
      if (!haveHigh) { hi = b; haveHigh = true; }
      else {
        display.pushIconPixel(((uint16_t)hi << 8) | b);
        haveHigh = false;
        pixelsDone++;
      }
    } else if (millis() - lastByte > 500) {
      display.endIconStream();
      return false;  // host died mid-transfer
    }
  }

  display.endIconStream();
  icon.markStreamed();
  return true;
}

void Channel::stopFader() {
  fader.stop();
}