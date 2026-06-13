//
// Created by Mackan on 2026-06-12.
// FaderFlow main — 5 channels, non-blocking serial parser, icon streaming.
//

#include "main.h"
#include <SPI.h>
#include <EEPROM.h>
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


// ---- Calibration ----
//
// Started by the host (CMD_CALIBRATION_START). For each channel in turn:
// pull the fader to the bottom, press its encoder knob, push it to the
// top, press again. Raw ADC ranges go to EEPROM and load at boot.

#define CAL_EEPROM_ADDR 32   // UUID lives at 0-16; plenty of clearance
#define CAL_EEPROM_MAGIC 0xCA

static bool calMode = false;
static uint8_t calChannel = 0;
static uint8_t calPhase = 0;  // 0 = waiting bottom, 1 = waiting top
static uint16_t calMinArr[NUM_CHANNELS];
static uint16_t calMaxArr[NUM_CHANNELS];
static uint16_t calBottomRaw = 0;            // bottom capture, echoed on the TOP prompt
static char     calMsg[NUM_CHANNELS][22];    // per-channel result, shown on the Done screen

static void sendCalStatus(uint8_t channel, uint8_t phase) {
  uint8_t msg[3] = { CMD_CALIBRATION_STATUS, channel, phase };
  Serial.write(msg, 3);
}

// Raw calibration values for the host's debug panel.
// kind 0 = bottom captured (v1=bottom raw); kind 1 = accepted (v1=min, v2=max);
// kind 2 = rejected (v1=travel). u16 sent little-endian.
static void sendCalDebug(uint8_t channel, uint8_t kind, uint16_t v1, uint16_t v2) {
  uint8_t msg[7] = {
    CMD_CALIBRATION_DEBUG, channel, kind,
    (uint8_t)(v1 & 0xFF), (uint8_t)(v1 >> 8),
    (uint8_t)(v2 & 0xFF), (uint8_t)(v2 >> 8)
  };
  Serial.write(msg, 7);
}

static void saveCalibration() {
  EEPROM.update(CAL_EEPROM_ADDR, CAL_EEPROM_MAGIC);
  int addr = CAL_EEPROM_ADDR + 1;
  for (uint8_t i = 0; i < NUM_CHANNELS; i++) {
    EEPROM.put(addr, calMinArr[i]); addr += 2;
    EEPROM.put(addr, calMaxArr[i]); addr += 2;
  }
}

static void loadCalibration() {
  for (uint8_t i = 0; i < NUM_CHANNELS; i++) {
    calMinArr[i] = FADER_RAW_MIN;
    calMaxArr[i] = FADER_RAW_MAX;
  }
  if (EEPROM.read(CAL_EEPROM_ADDR) != CAL_EEPROM_MAGIC) return;
  int addr = CAL_EEPROM_ADDR + 1;
  for (uint8_t i = 0; i < NUM_CHANNELS; i++) {
    uint16_t mn, mx;
    EEPROM.get(addr, mn); addr += 2;
    EEPROM.get(addr, mx); addr += 2;
    if (mx > mn && mx <= 1023 && (mx - mn) > 200) {
      calMinArr[i] = mn;
      calMaxArr[i] = mx;
      if (i < NUM_CONNECTED_CHANNELS) channels[i]->setFaderCalibration(mn, mx);
    }
  }
}

static void showCalScreen(uint8_t i) {
  if (i >= NUM_CONNECTED_CHANNELS) return;
  if (i < calChannel) {
    // Already calibrated: show this channel's captured result on screen
    channels[i]->showMessage("CALIBRATE", "Done!", calMsg[i]);
  } else if (i == calChannel) {
    if (calPhase == 0) {
      channels[i]->showMessage("CALIBRATE", "Fader to BOTTOM", "then press knob");
    } else {
      // Phase 1: echo the bottom value just captured so it can be eyeballed
      char l3[22];
      snprintf(l3, sizeof(l3), "btm=%u, press", (unsigned)calBottomRaw);
      channels[i]->showMessage("CALIBRATE", "Fader to TOP", l3);
    }
  } else {
    channels[i]->showMessage("CALIBRATE", "Waiting...", nullptr);
  }
}

static void startCalibration() {
  // Coast, not brake -- the user must hand-position each fader against the
  // physical stops, and an electrical brake makes that stiff and unstable.
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) channels[i]->releaseFader();
  calMode = true;
  calChannel = 0;
  calPhase = 0;
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) showCalScreen(i);
  sendCalStatus(0, 0);
}

static void exitCalibration(uint8_t statusPhase) {
  calMode = false;
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    channels[i]->flushInputs();   // discard knob twiddling during cal
    channels[i]->releaseFader();  // coast so faders stay hand-movable until host re-seeks
    channels[i]->redrawUI();
  }
  sendCalStatus(0, statusPhase);  // host re-syncs names/icons/volumes
}

static void cancelCalibration() {
  if (!calMode) return;
  loadCalibration();  // discard partial captures, restore saved values
  exitCalibration(3);
}

static void runCalibration() {
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    channels[i]->pollEncoderButton();
  }

  bool pressed = false;
  for (uint8_t i = 0; i < NUM_CONNECTED_CHANNELS; i++) {
    if (channels[i]->wasButtonPressed()) pressed = true;
  }


  if (!pressed) return;

  if (calPhase == 0) {
    calBottomRaw = channels[calChannel]->faderRaw();
    sendCalDebug(calChannel, 0, calBottomRaw, 0);
    calPhase = 1;
    showCalScreen(calChannel);
    sendCalStatus(calChannel, 1);
  } else {
    uint16_t topRaw = channels[calChannel]->faderRaw();
    uint16_t mn = min(calBottomRaw, topRaw);
    uint16_t mx = max(calBottomRaw, topRaw);
    uint16_t travel = mx - mn;
    uint8_t done = calChannel;
    if (travel > 200) {  // sanity: the fader actually traveled
      calMinArr[calChannel] = mn;
      calMaxArr[calChannel] = mx;
      channels[calChannel]->setFaderCalibration(mn, mx);
      snprintf(calMsg[done], sizeof(calMsg[done]), "%u-%u OK", (unsigned)mn, (unsigned)mx);
      sendCalDebug(done, 1, mn, mx);
    } else {
      snprintf(calMsg[done], sizeof(calMsg[done]), "t=%u REJECT", (unsigned)travel);
      sendCalDebug(done, 2, travel, 0);
    }
    calPhase = 0;
    calChannel++;
    showCalScreen(done);  // now shows Done!
    if (calChannel >= NUM_CONNECTED_CHANNELS) {
      saveCalibration();
      exitCalibration(2);
    } else {
      showCalScreen(calChannel);
      sendCalStatus(calChannel, 0);
    }
  }
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
    case CMD_CALIBRATION_START:
    case CMD_CALIBRATION_CANCEL:
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

  if (!calMode && ch < NUM_CONNECTED_CHANNELS) {
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
    if (!calMode && c.channel < NUM_CONNECTED_CHANNELS) {
      channels[c.channel]->setApp(c.name);
    }
  }
  else if (cmd == CMD_DISPLAY_UPDATE_APP_VOLUME) {
    DisplayUpdateVolumeCommand c;
    memcpy(&c, rxBuf, sizeof(c));
    if (!calMode && c.channel < NUM_CONNECTED_CHANNELS) {
      // Updates the display AND motor-seeks the fader. The fader
      // suppresses touch events during the seek, so this won't echo
      // back to the host as a CMD_FADER_UPDATE.
      channels[c.channel]->setVolume(c.volume);
    }
  }
  else if (cmd == CMD_DISPLAY_UPDATE_ICON) {
    handleIconTransfer(rxBuf[1]);
  }
  else if (cmd == CMD_CALIBRATION_START) {
    startCalibration();
  }
  else if (cmd == CMD_CALIBRATION_CANCEL) {
    cancelCalibration();
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

  loadCalibration();
}

void loop() {
  static unsigned long lastBeacon = 0;

  // Auto-announce until the desktop app answers
  if (!handshakeComplete && millis() - lastBeacon > 500) {
    sendHandshake();
    lastBeacon = millis();
  }

  pumpSerial();

  if (calMode) {
    runCalibration();
    return;
  }

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