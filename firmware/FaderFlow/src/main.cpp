//
// Created by Mackan on 2026-06-12.
// FaderFlow main — 5 channels, non-blocking serial parser, icon streaming.
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

// ---- Outgoing ----

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

// ---- Incoming: non-blocking packet assembler ----
//
// Bytes are pumped into rxBuf as they arrive; a command is dispatched
// only once its complete packet is present. A truncated packet just
// waits in the buffer instead of desyncing the stream via readBytes
// timeouts. Unknown lead bytes are discarded one at a time (resync).
//
// Exception: CMD_DISPLAY_UPDATE_ICON. Its 8192-byte payload doesn't fit
// any buffer on this chip, so the parser treats the 2-byte header as the
// packet and the handler streams the payload straight to the display.

#define RX_BUF_SIZE 80  // largest buffered packet: DisplayUpdateAppCommand (66 B)

static uint8_t rxBuf[RX_BUF_SIZE];
static uint8_t rxLen = 0;
static uint8_t rxExpected = 0;

// Full packet length (incl. cmd byte) for each command. 0 = unknown.
static uint8_t packetLength(uint8_t cmd) {
  switch (cmd) {
    case CMD_HANDSHAKE_REQUEST:
    case CMD_HANDSHAKE_ACK:
    case CMD_ECHO_UUID:
      return 1;
    case CMD_DISPLAY_UPDATE_APP_NAME:
      return sizeof(DisplayUpdateAppCommand);
    case CMD_DISPLAY_UPDATE_APP_VOLUME:
      return sizeof(DisplayUpdateVolumeCommand);
    case CMD_DISPLAY_UPDATE_ICON:
      return 2;  // header only — payload is streamed by the handler
    default:
      if (cmd == 'h' || cmd == 'u') return 1;  // debug shortcuts
      return 0;
  }
}

static void handleIconTransfer(uint8_t ch) {
  // Motors must not run unsupervised during the ~0.75s blocking transfer
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) channels[i]->stopFader();

  if (ch < NUM_CONNECTED_CHANNELS) {
    channels[ch]->receiveIcon(Serial);
  } else {
    // Invalid channel: still consume the payload or the stream desyncs
    uint16_t remaining = 8192;
    uint32_t lastByte = millis();
    while (remaining > 0 && millis() - lastByte < 500) {
      if (Serial.available()) { Serial.read(); remaining--; lastByte = millis(); }
    }
  }
}

static void dispatchPacket() {
  uint8_t cmd = rxBuf[0];

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
    DisplayUpdateAppCommand c;
    memcpy(&c, rxBuf, sizeof(c));
    c.name[63] = '\0';
    if (c.channel < NUM_CONNECTED_CHANNELS) {
      channels[c.channel]->setApp(c.name);
    }
  }
  else if (cmd == CMD_DISPLAY_UPDATE_APP_VOLUME) {
    DisplayUpdateVolumeCommand c;
    memcpy(&c, rxBuf, sizeof(c));
    if (c.channel < NUM_CONNECTED_CHANNELS) {
      // Updates the display AND motor-seeks the fader. The fader
      // suppresses touch events during the seek, so this won't echo
      // back to the host as a CMD_FADER_UPDATE.
      channels[c.channel]->setVolume(c.volume);
    }
  }
  else if (cmd == CMD_DISPLAY_UPDATE_ICON) {
    handleIconTransfer(rxBuf[1]);
  }
}

static void pumpSerial() {
  while (Serial.available() > 0) {
    uint8_t b = Serial.read();

    if (rxLen == 0) {
      // Start of a packet: byte must be a known command
      rxExpected = packetLength(b);
      if (rxExpected == 0) continue;  // unknown byte — discard, resync
    }

    rxBuf[rxLen++] = b;

    if (rxLen >= rxExpected) {
      dispatchPacket();
      rxLen = 0;
      rxExpected = 0;
    }
  }
}

// ---- Arduino ----

void setup() {
  Serial.begin(115200);

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
    pumpSerial();  // drain anything the host sent during slow display init
  }
}

void loop() {
  static unsigned long lastBeacon = 0;

  // Auto-announce until the desktop app answers
  if (!handshakeComplete && millis() - lastBeacon > 500) {
    sendHandshake();
    lastBeacon = millis();
  }

  pumpSerial();

  // Local hardware always runs (encoders, touch detection,
  // motor seeks, display updates)
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    Channel* ch = channels[i];
    ch->update();

    pumpSerial();  // drain between channels — display redraws are slow

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