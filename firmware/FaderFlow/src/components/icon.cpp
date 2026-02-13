//
// Created by Mackan on 2026-02-13.
//

#include "icon.h"

// Line buffer for drawing
uint16_t Icon::lineBuffer[ICON_SIZE];

Icon::Icon() {
  ready = false;
  usingTestIcon = false;
  highByte = 0;
  expectingLowByte = false;
  pixelIndex = 0;
}

void Icon::startReceiving() {
  ready = false;
  usingTestIcon = false;
  highByte = 0;
  expectingLowByte = false;
  pixelIndex = 0;
}

void Icon::addByte(uint8_t byte) {
  // This would be used when receiving icons over serial
  if (!expectingLowByte) {
    highByte = byte;
    expectingLowByte = true;
  } else {
    pixelIndex++;
    expectingLowByte = false;

    if (pixelIndex >= ICON_SIZE * ICON_SIZE) {
      ready = true;
      usingTestIcon = false;
    }
  }
}

bool Icon::isReady() {
  return ready || usingTestIcon;
}

void Icon::draw(Adafruit_GFX* display, int16_t x, int16_t y) {
  if (!isReady()) return;

  if (usingTestIcon) {
    // Draw a simple procedural test icon (cyan speaker)
    uint16_t cyan = 0x07FF;
    uint16_t black = 0x0000;

    // Speaker body
    display->fillRect(x + 10, y + 20, 15, 24, cyan);

    // Speaker cone
    display->fillTriangle(x + 25, y + 20, x + 25, y + 44, x + 40, y + 50, cyan);
    display->fillTriangle(x + 25, y + 20, x + 25, y + 44, x + 40, y + 14, cyan);

    // Sound waves
    for (int i = 0; i < 3; i++) {
      int offset = i * 6;
      display->drawCircle(x + 30, y + 32, 18 + offset, cyan);
    }
  }
}

void Icon::useTestIcon() {
  usingTestIcon = true;
  ready = true;
}

size_t Icon::getBufferSize() {
  return ICON_SIZE * ICON_SIZE * sizeof(uint16_t);
}

void Icon::clear() {
  ready = false;
  usingTestIcon = false;
  pixelIndex = 0;
}