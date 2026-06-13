//
// Created by Mackan on 2026-02-11.
// Reworked 2026-06-12: class-based, motor control merged in, protocol decoupled.
//

#include "fader.h"

Fader::Fader(uint8_t motorA, uint8_t motorB, uint8_t analogPin)
  : motorA(motorA), motorB(motorB), analogPin(analogPin) {
  calMin = FADER_RAW_MIN;
  calMax = FADER_RAW_MAX;
  lastRawValue = -1;
  lastReported = -1;
  lastReadTime = 0;
  target = -1;
  seeking = false;
  seekStart = 0;
  prevSeekPos = 0;
  lastVelSample = 0;
  velocity = 0;
  settleUntil = 0;
  seekRetries = 0;
  crawlBoost = 0;
  stallCount = 0;
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
  // Pots never reach the rails at their physical stops — map the
  // calibrated usable range so 0% and 100% are reachable positions.
  // FADER_END_MARGIN pulls each endpoint a few counts inside the captured
  // stop: press-time capture is a firmer push than normal use, so a typical
  // hand-push lands a few counts short and would read 1%/99% without it.
  int lo = calMin + FADER_END_MARGIN;   // toward the 100% stop
  int hi = calMax - FADER_END_MARGIN;   // toward the 0% stop
  if (hi <= lo) { lo = calMin; hi = calMax; }  // range too small for margin
  raw = constrain(raw, lo, hi);
#if FADER_INVERTED
  return map(raw, hi, lo, 0, 100);
#else
  return map(raw, lo, hi, 0, 100);
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

int Fader::readRawAveraged() {
  // ADC mux settle, then average — from the bring-up sketch
  analogRead(analogPin);
  delayMicroseconds(100);
  long sum = 0;
  for (uint8_t k = 0; k < 8; k++) sum += analogRead(analogPin);
  return sum / 8;
}

int Fader::read() {
  return rawToPercent(readRawAveraged());
}

void Fader::setTarget(int percent) {
  percent = constrain(percent, 0, 100);

  // Already there? Don't twitch the motor.
  if (abs(getPosition() - percent) <= FADER_SEEK_DEADBAND) return;

  target = percent;
  seeking = true;
  seekStart = millis();
  prevSeekPos = getPosition();
  lastVelSample = millis();
  velocity = 0;
  settleUntil = 0;
  seekRetries = 0;
  crawlBoost = 0;
  stallCount = 0;
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
  settleUntil = 0;
}

void Fader::release() {
  // Coast (both inputs LOW) instead of brake (both HIGH). motorWrite(0)
  // brakes -- correct for crisply ending a seek, wrong for letting the
  // user hand-position the fader, which is exactly what calibration needs.
  analogWrite(motorA, 0);
  analogWrite(motorB, 0);
  seeking = false;
  target = -1;
  settleUntil = 0;
}

void Fader::update() {
  unsigned long now = millis();

  if (seeking) {
    // Seek loop runs unthrottled for responsive control
    int pos = read();

    // Post-brake settle: let the mechanics come to rest, then verify
    if (settleUntil) {
      if (now < settleUntil) return;
      settleUntil = 0;
      int err = target - pos;
      if (abs(err) <= FADER_SEEK_DEADBAND || seekRetries >= FADER_SEEK_MAX_RETRIES) {
        // Arrived (or close enough after max retries) — finish
        seeking = false;
        target = -1;

        // Re-sync state so the motor's own movement doesn't
        // register as a user touch and echo back to the host
        lastRawValue = analogRead(analogPin);
        lastReported = rawToPercent(lastRawValue);
        return;
      }
      // Bounced off target/end stop — creep back
      seekRetries++;
      crawlBoost = 0;
      seekStart = now;  // fresh timeout for the correction pass
    }

    int err = target - pos;

    // Velocity estimate + anti-stall, sampled every 8 ms
    if (now - lastVelSample >= 8) {
      velocity = pos - prevSeekPos;
      prevSeekPos = pos;
      lastVelSample = now;
      if (velocity == 0) {
        // Not moving: bump PWM until it breaks free...
        if (crawlBoost < 60) crawlBoost += 5;
        // ...but stalled AT max boost means a physical end stop.
        // Wherever the stop is, it IS the end — accept and finish
        // instead of grinding the motor into the wall until timeout.
        else if (++stallCount >= FADER_STALL_SAMPLES) {
          motorWrite(0);
          settleUntil = now + FADER_SEEK_SETTLE_MS;
          seekRetries = FADER_SEEK_MAX_RETRIES;  // no correction pass
          return;
        }
      } else {
        crawlBoost = 0;
        stallCount = 0;
      }
    }

    // Brake when arrived, when momentum will cross the target this
    // sample anyway (predictive stop), or on timeout
    bool arrived  = abs(err) <= FADER_SEEK_DEADBAND;
    bool crossing = velocity != 0 && ((err > 0) == (velocity > 0))
                    && abs(err) <= abs(velocity);
    if (arrived || crossing || now - seekStart > FADER_SEEK_TIMEOUT) {
      motorWrite(0);  // brake
      settleUntil = now + FADER_SEEK_SETTLE_MS;
      return;
    }

    // Speed profile: full proportional far out, short ramp-down near
    // the target (predictive brake handles the rest). Correction
    // passes always creep.
    int speed;
    if (seekRetries > 0) {
      speed = FADER_SEEK_CRAWL + crawlBoost;
    } else if (abs(err) <= FADER_SEEK_SLOW_ZONE) {
      speed = map(abs(err), 0, FADER_SEEK_SLOW_ZONE,
                  FADER_SEEK_CRAWL, FADER_SEEK_MIN_SPEED) + crawlBoost;
    } else {
      speed = constrain(abs(err) * 6, FADER_SEEK_MIN_SPEED, 255);
    }
    motorWrite(err > 0 ? speed : -speed);
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

void Fader::setCalibration(int rawMin, int rawMax) {
  if (rawMax - rawMin < 100) return;  // refuse nonsense ranges
  calMin = rawMin;
  calMax = rawMax;
  // Re-prime so the rescaled position does not fire a touch event
  lastRawValue = analogRead(analogPin);
  lastReported = rawToPercent(lastRawValue);
  moved = false;
}