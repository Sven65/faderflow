//
// Created by Mackan on 2026-02-13.
//

#include "icon.h"

Icon::Icon() {
    ready = false;
    highByte = 0;
    expectingLowByte = false;
    pixelIndex = 0;
    clear();
}

void Icon::startReceiving() {
    ready = false;
    highByte = 0;
    expectingLowByte = false;
    pixelIndex = 0;
}

void Icon::addByte(uint8_t byte) {
    if (!expectingLowByte) {
        // This is the high byte of RGB565
        highByte = byte;
        expectingLowByte = true;
    } else {
        // This is the low byte, combine with high byte
        uint16_t pixel = (highByte << 8) | byte;

        if (pixelIndex < ICON_SIZE * ICON_SIZE) {
            buffer[pixelIndex] = pixel;
            pixelIndex++;

            // Check if we've received all pixels
            if (pixelIndex >= ICON_SIZE * ICON_SIZE) {
                ready = true;
            }
        }

        expectingLowByte = false;
    }
}

bool Icon::isReady() {
    return ready;
}

void Icon::draw(Adafruit_GFX* display, int16_t x, int16_t y) {
    if (ready) {
        display->drawRGBBitmap(x, y, buffer, ICON_SIZE, ICON_SIZE);
    }
}

uint16_t* Icon::getBuffer() {
    return buffer;
}

size_t Icon::getBufferSize() {
    return ICON_SIZE * ICON_SIZE * sizeof(uint16_t);
}

void Icon::clear() {
    for (int i = 0; i < ICON_SIZE * ICON_SIZE; i++) {
        buffer[i] = 0;
    }
    ready = false;
    pixelIndex = 0;
}