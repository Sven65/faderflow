//
// Created by Mackan on 2026-02-11.
// Reworked 2026-06-12: class-based, motor control merged in, protocol decoupled.
//

#include "fader.h"

Fader::Fader(uint8_t motorA, uint8_t motorB, uint8_t analogPin)
  : motorA(motorA), motorB(motorB), analogPin(analogPin) {
  lastRawValue = -1;
  lastReported = -1;
  lastReadTime = 0;
  target = -1;
  seeking = false;
  seekStart = 0;
  moved = false;
}

void Fader::begin() {
  pinMode(analogPin, INPUT);
  pinMode(motorA, OUTPUT);
  pinMode(motorB, OUTPUT);
  motorWrite(0);

  // Prime the EMA and reported value so we don't fire a
  // spurious move event on the first update()
  lastRawValue = analogRead(analogPin);
  lastReported = rawToPercent(lastRawValue);
}

int Fader::rawToPercent(int raw) {
#if FADER_INVERTED
  return map(raw, 1023, 0, 0, 100);
#else
  return map(raw, 0, 1023, 0, 100);
#endif
}

void Fader::motorWrite(int speed) {
  speed = constrain(speed, -255, 255);
  if (speed > 0) {
    analogWrite(motorA, speed);
    analogWrite(motorB, 0);
  } else if (speed < 0) {
    analogWrite(motorA, 0);
    analogWrite(motorB, -speed);
  } else {
    // Brake (both high)
    analogWrite(motorA, 255);
    analogWrite(motorB, 255);
  }
}

int Fader::read() {
  // ADC mux settle, then average — from the bring-up sketch
  analogRead(analogPin);
  delayMicroseconds(100);
  long sum = 0;
  for (uint8_t k = 0; k < 8; k++) sum += analogRead(analogPin);
  return rawToPercent(sum / 8);
}

void Fader::setTarget(int percent) {
  percent = constrain(percent, 0, 100);

  // Already there? Don't twitch the motor.
  if (abs(getPosition() - percent) <= FADER_SEEK_DEADBAND) return;

  target = percent;
  seeking = true;
  seekStart = millis();
}

bool Fader::isSeeking() {
  return seeking;
}

int Fader::getPosition() {
  if (lastRawValue == -1) return 0;
  return rawToPercent(lastRawValue);
}

void Fader::stop() {
  motorWrite(0);
  seeking = false;
  target = -1;
}

void Fader::update() {
  unsigned long now = millis();

  if (seeking) {
    // Seek loop runs unthrottled for responsive control
    int pos = read();
    int err = target - pos;

    if (abs(err) <= FADER_SEEK_DEADBAND || now - seekStart > FADER_SEEK_TIMEOUT) {
      motorWrite(0);  // brake
      seeking = false;
      target = -1;

      // Re-sync state so the motor's own movement doesn't
      // register as a user touch and echo back to the host
      lastRawValue = analogRead(analogPin);
      lastReported = rawToPercent(lastRawValue);
      return;
    }

    // Proportional with a friction floor
    int speed = err * 6;
    if (speed > 0) speed = constrain(speed, FADER_SEEK_MIN_SPEED, 255);
    else speed = constrain(speed, -255, -FADER_SEEK_MIN_SPEED);
    motorWrite(speed);
    return;
  }

  // Idle: rate-limited touch detection
  if (now - lastReadTime < FADER_READ_INTERVAL) return;
  lastReadTime = now;

  // Burst average: noise reduction without lag — all samples are "now"
  analogRead(analogPin);
  delayMicroseconds(100);
  long sum = 0;
  for (uint8_t k = 0; k < 4; k++) sum += analogRead(analogPin);
  int rawValue = sum / 4;

  if (lastRawValue == -1) lastRawValue = rawValue;
  else lastRawValue = (lastRawValue + rawValue) / 2;

  int pos = rawToPercent(lastRawValue);

  if (abs(pos - lastReported) > FADER_DEADBAND) {
    lastReported = pos;
    moved = true;
  }
}

bool Fader::hasMoved() {
  bool m = moved;
  moved = false;
  return m;
}