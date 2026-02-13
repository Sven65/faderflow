#ifndef DEVICE_ID_H
#define DEVICE_ID_H

#include <Arduino.h>

#define UUID_SIZE 16

void initDeviceID();
void getDeviceUUID(uint8_t* uuid);

#endif