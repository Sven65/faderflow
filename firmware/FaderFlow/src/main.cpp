//
// Created by Mackan on 2026-06-12.
// FaderFlow main — 5 channels via Channel API, serial protocol + handshake.
//

#include "main.h"
#include <SPI.h>
#include "protocol.h"
#include "utils/device_id.h"
#include "utils/comms.h"
#include "channel.h"

#define NUM_CHANNELS 5
#define NUM_CONNECTED_CHANNELS 5
#define BL_PIN 12
#define SS_PIN 53

// Per-channel pin tables (Mega). Each channel owns a 5-pin digital block:
// CS, DC, CLK, DT, SW — then RST on A9-A13.
const int8_t  CS_PINS[NUM_CHANNELS]   = { 22, 27, 32, 37, 42 };
const int8_t  DC_PINS[NUM_CHANNELS]   = { 23, 28, 33, 38, 43 };
const int8_t  RST_PINS[NUM_CHANNELS]  = { 63, 64, 65, 66, 67 };  // A9-A13
const uint8_t ENC_CLK[NUM_CHANNELS]   = { 24, 29, 34, 39, 44 };
const uint8_t ENC_DT[NUM_CHANNELS]    = { 25, 30, 35, 40, 45 };
const uint8_t ENC_SW[NUM_CHANNELS]    = { 26, 31, 36, 41, 46 };
const uint8_t MOTOR_A[NUM_CHANNELS]   = { 2, 4, 6, 8, 10 };      // all PWM
const uint8_t MOTOR_B[NUM_CHANNELS]   = { 3, 5, 7, 9, 11 };
const uint8_t FADER_PIN[NUM_CHANNELS] = { A0, A1, A2, A3, A4 };

Channel* channels[NUM_CHANNELS];

static bool handshakeComplete = false;

// Protocol speaks 0-255 for fader position; channels speak 0-100
static uint8_t volumeToProtocol(int volume) {
  return map(volume, 0, 100, 0, 255);
}

static void sendFaderUpdate(uint8_t channel, int volume) {
  FaderMessage msg;
  msg.cmd = CMD_FADER_UPDATE;
  msg.channel = channel;
  msg.position = volumeToProtocol(volume);
  Serial.write((uint8_t*)&msg, sizeof(msg));
}

// ---- Incoming command handlers ----

static void handleAppNameUpdate() {
  DisplayUpdateAppCommand cmd;
  // cmd byte already consumed — read from .channel onward
  size_t bytesToRead = sizeof(DisplayUpdateAppCommand) - 1;
  if (Serial.readBytes((uint8_t*)&cmd.channel, bytesToRead) != bytesToRead) {
    return;
  }
  cmd.name[63] = '\0';  // force null termination

  if (cmd.channel >= NUM_CONNECTED_CHANNELS) return;
  channels[cmd.channel]->setApp(cmd.name);
}

static void handleVolumeUpdate() {
  DisplayUpdateVolumeCommand cmd;
  size_t bytesToRead = sizeof(DisplayUpdateVolumeCommand) - 1;
  if (Serial.readBytes((uint8_t*)&cmd.channel, bytesToRead) != bytesToRead) {
    return;
  }

  if (cmd.channel >= NUM_CONNECTED_CHANNELS) return;
  // setVolume updates the display AND motor-seeks the fader.
  // Fader suppresses touch events during the seek, so this
  // won't echo back to the host as a CMD_FADER_UPDATE.
  channels[cmd.channel]->setVolume(cmd.volume);
}

static void handleSerialCommand() {
  uint8_t cmd = Serial.read();

  if (cmd == CMD_HANDSHAKE_REQUEST || cmd == 'h') {
    sendHandshake();
    handshakeComplete = true;
  }
  else if (cmd == CMD_HANDSHAKE_ACK) {
    handshakeComplete = true;  // stop beaconing
  }
  else if (cmd == CMD_ECHO_UUID || cmd == 'u') {
    uint8_t uuid[UUID_SIZE];
    getDeviceUUID(uuid);
    Serial.write(uuid, UUID_SIZE);
  }
  else if (cmd == CMD_DISPLAY_UPDATE_APP_NAME) {
    handleAppNameUpdate();
  }
  else if (cmd == CMD_DISPLAY_UPDATE_APP_VOLUME) {
    handleVolumeUpdate();
  }
}

void setup() {
  Serial.begin(115200);
  Serial.setTimeout(50);

  initDeviceID();

  pinMode(BL_PIN, OUTPUT);
  analogWrite(BL_PIN, 200);

  // Keep the Mega in SPI master mode
  pinMode(SS_PIN, OUTPUT);
  digitalWrite(SS_PIN, HIGH);

  // All CS lines high BEFORE any display init — always all 5,
  // even if fewer channels are active, so floating CS lines
  // can't latch stray traffic on the shared bus
  for (uint8_t i = 0; i < NUM_CHANNELS; i++) {
    pinMode(CS_PINS[i], OUTPUT);
    digitalWrite(CS_PINS[i], HIGH);
  }

  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    channels[i] = new Channel(
      i,
      CS_PINS[i], DC_PINS[i], RST_PINS[i],
      ENC_DT[i], ENC_CLK[i], ENC_SW[i],
      MOTOR_A[i], MOTOR_B[i], FADER_PIN[i]
    );
    channels[i]->begin();
  }
}

void loop() {
  static unsigned long lastBeacon = 0;

  // Auto-announce until the desktop app answers
  if (!handshakeComplete && millis() - lastBeacon > 500) {
    sendHandshake();
    lastBeacon = millis();
  }

  if (Serial.available() > 0) {
    handleSerialCommand();
  }

  // Local hardware always runs (encoders, touch detection,
  // motor seeks, display updates)
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    Channel* ch = channels[i];
    ch->update();

    // ...but only report to the host once the link is up
    if (!handshakeComplete) continue;

    if (ch->hasFaderChanged()) {
      sendFaderUpdate(i, ch->getVolume());
    }

    if (ch->hasEncoderChanged()) {
      ch->getEncoderDelta();  // consume
      sendFaderUpdate(i, ch->getVolume());
    }

    if (ch->wasButtonPressed()) {
      // TODO: mute toggle / button CMD
    }
  }
}