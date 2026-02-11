//
// Created by Mackan on 2026-02-11.
//

#ifndef FADERFLOW_FADER_H
#define FADERFLOW_FADER_H

#include <Arduino.h>

#define NUM_FADERS 5
#define NUM_CONNECTED_FADERS 2
#define FADER_DEADBAND 2
#define FADER_READ_INTERVAL 20

// Fader pin assignments
#define FADER_1_PIN A0
#define FADER_2_PIN A1
#define FADER_3_PIN A2
#define FADER_4_PIN A3
#define FADER_5_PIN A4

void initFaders();
void readFaders();

#endif //FADERFLOW_FADER_H