#include "device_id.h"
#include <EEPROM.h>

#define UUID_ADDR 0
#define UUID_MAGIC 0xAB

void initDeviceID() {
    if (EEPROM.read(UUID_ADDR) != UUID_MAGIC) {
        // Generate UUID on first boot
        EEPROM.write(UUID_ADDR, UUID_MAGIC);
        randomSeed(analogRead(A7));
        for (int i = 0; i < UUID_SIZE; i++) {
            EEPROM.write(UUID_ADDR + 1 + i, random(256));
        }
    }
}

void getDeviceUUID(uint8_t* uuid) {
    for (int i = 0; i < UUID_SIZE; i++) {
        uuid[i] = EEPROM.read(UUID_ADDR + 1 + i);
    }
}