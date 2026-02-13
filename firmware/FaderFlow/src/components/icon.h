//
// Created by Mackan on 2026-02-13.
//

#ifndef FADERFLOW_ICON_H
#define FADERFLOW_ICON_H

#include <Arduino.h>
#include <Adafruit_GFX.h>

#define ICON_SIZE 64

class Icon {
  public:
    Icon();

    // Start receiving a new icon
    void startReceiving();

    // Add a byte of icon data
    void addByte(uint8_t byte);

    // Check if icon is ready to display
    bool isReady();

    // Draw the icon at the specified position
    void draw(Adafruit_GFX* display, int16_t x, int16_t y);

    // Use the test icon from PROGMEM
    void useTestIcon();

    // Get buffer size in bytes
    static size_t getBufferSize();

    // Clear the icon
    void clear();

  private:
    bool ready;
    bool usingTestIcon;
    uint8_t highByte;
    bool expectingLowByte;
    int pixelIndex;

    // Small line buffer for drawing from PROGMEM
    static uint16_t lineBuffer[ICON_SIZE];
};

#endif //FADERFLOW_ICON_H