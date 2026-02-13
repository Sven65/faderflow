//
// Created by Mackan on 2026-02-13.
//

#ifndef FADERFLOW_DISPLAY_H
#define FADERFLOW_DISPLAY_H

#include <Adafruit_GFX.h>
#include <Adafruit_ST7789.h>
#include "icon.h"

// Screen dimensions
#define SCREEN_WIDTH  240
#define SCREEN_HEIGHT 240

// UI Colors
#define BG_COLOR      0x0000  // Black
#define ICON_BG       0x2124  // Dark gray
#define TEXT_COLOR    0xFFFF  // White
#define BAR_BG        0x2124  // Dark gray
#define BAR_FILL      0x07FF  // Cyan
#define ACCENT_COLOR  0x07FF  // Cyan

class Display {
public:
    Display(int8_t cs, int8_t dc, int8_t rst);

    // Initialize the display
    void begin();

    // Draw the complete UI
    void drawUI(int volume, const char* appName, Icon* icon);

    // Update only the volume display (efficient)
    void updateVolume(int volume);

    // Update only the app name
    void updateAppName(const char* appName);

    // Update only the icon
    void updateIcon(Icon* icon);

    // Get the underlying TFT object if needed
    Adafruit_ST7789* getTFT();

private:
    Adafruit_ST7789 tft;
    int currentVolume;

    void drawVolumeDisplay(int volume);
    void drawPlaceholderIcon(int x, int y, int size);
};

#endif //FADERFLOW_DISPLAY_H