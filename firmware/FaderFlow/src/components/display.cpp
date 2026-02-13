//
// Created by Mackan on 2026-02-13.
//

#include "display.h"
#include "../config.h"

Display::Display(int8_t cs, int8_t dc, int8_t rst)
  : tft(cs, dc, SHARED_MOSI_PIN, SHARED_SCLK_PIN, rst) {
  currentVolume = 0;
}

void Display::begin() {
  tft.init(SCREEN_WIDTH, SCREEN_HEIGHT);
  tft.setRotation(2); // Adjust as needed
  tft.fillScreen(BG_COLOR);
}

void Display::drawUI(int volume, const char* appName, Icon* icon) {
  currentVolume = volume;

  // Clear screen
  tft.fillScreen(BG_COLOR);

  // Draw icon area (centered, upper third)
  int iconX = (SCREEN_WIDTH - ICON_SIZE) / 2;
  int iconY = 40;

  // Icon background
  tft.fillRoundRect(iconX - 8, iconY - 8, ICON_SIZE + 16, ICON_SIZE + 16, 8, ICON_BG);

  // Draw icon if we have one, otherwise show placeholder
  if (icon && icon->isReady()) {
    icon->draw(&tft, iconX, iconY);
  } else {
    drawPlaceholderIcon(iconX, iconY, ICON_SIZE);
  }

  // Draw app name
  tft.setTextColor(TEXT_COLOR);
  tft.setTextSize(2);
  int16_t x1, y1;
  uint16_t w, h;
  tft.getTextBounds(appName, 0, 0, &x1, &y1, &w, &h);
  int textX = (SCREEN_WIDTH - w) / 2;
  tft.setCursor(textX, iconY + ICON_SIZE + 20);
  tft.print(appName);

  // Draw volume display
  drawVolumeDisplay(volume);
}

void Display::updateVolume(int volume) {
  currentVolume = volume;
  // Clear volume area
  tft.fillRect(0, 140, SCREEN_WIDTH, 100, BG_COLOR);
  drawVolumeDisplay(volume);
}

void Display::updateAppName(const char* appName) {
  int iconY = 40;

  // Clear app name area - between icon and volume display
  tft.fillRect(0, iconY + ICON_SIZE + 10, SCREEN_WIDTH, 30, BG_COLOR);
  //                                                      ^^ Back to 30 height

  // Draw new app name
  tft.setTextColor(TEXT_COLOR);
  tft.setTextSize(2);
  int16_t x1, y1;
  uint16_t w, h;
  tft.getTextBounds(appName, 0, 0, &x1, &y1, &w, &h);
  int textX = (SCREEN_WIDTH - w) / 2;
  tft.setCursor(textX, iconY + ICON_SIZE + 15);  // <-- Move text UP to match clear area
  tft.print(appName);
}

void Display::updateIcon(Icon* icon) {
  int iconX = (SCREEN_WIDTH - ICON_SIZE) / 2;
  int iconY = 40;

  // Clear icon area
  tft.fillRoundRect(iconX - 8, iconY - 8, ICON_SIZE + 16, ICON_SIZE + 16, 8, ICON_BG);

  // Draw icon
  if (icon && icon->isReady()) {
    icon->draw(&tft, iconX, iconY);
  } else {
    drawPlaceholderIcon(iconX, iconY, ICON_SIZE);
  }
}

void Display::drawVolumeDisplay(int volume) {
  // Volume percentage - large and centered
  tft.setTextColor(TEXT_COLOR);
  tft.setTextSize(4);

  char volStr[8];
  sprintf(volStr, "%d%%", volume);

  int16_t x1, y1;
  uint16_t w, h;
  tft.getTextBounds(volStr, 0, 0, &x1, &y1, &w, &h);
  int textX = (SCREEN_WIDTH - w) / 2;
  tft.setCursor(textX, 150);
  tft.print(volStr);

  // Volume bar
  int barWidth = 200;
  int barHeight = 12;
  int barX = (SCREEN_WIDTH - barWidth) / 2;
  int barY = 200;

  // Bar background
  tft.fillRoundRect(barX, barY, barWidth, barHeight, 6, BAR_BG);

  // Bar fill
  int fillWidth = (barWidth - 4) * volume / 100;
  if (fillWidth > 0) {
    tft.fillRoundRect(barX + 2, barY + 2, fillWidth, barHeight - 4, 4, BAR_FILL);
  }
}

void Display::drawPlaceholderIcon(int x, int y, int size) {
  // Simple speaker icon placeholder
  tft.fillRect(x + 10, y + 20, 15, 24, ACCENT_COLOR);
  tft.fillTriangle(x + 25, y + 20, x + 25, y + 44, x + 40, y + 50, ACCENT_COLOR);
  tft.fillTriangle(x + 25, y + 20, x + 25, y + 44, x + 40, y + 14, ACCENT_COLOR);

  // Sound waves
  for (int i = 0; i < 3; i++) {
    int offset = i * 6;
    tft.drawCircle(x + 30, y + 32, 18 + offset, ACCENT_COLOR);
  }
}

Adafruit_ST7789* Display::getTFT() {
  return &tft;
}