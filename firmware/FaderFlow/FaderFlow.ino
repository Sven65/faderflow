#define ENC_A 2
#define ENC_B 3
#define ENC_SW 4

volatile int encoderPos = 0;
volatile int lastState = 0;

void setup() {
  Serial.begin(9600);
  pinMode(ENC_A, INPUT);
  pinMode(ENC_B, INPUT);
  pinMode(ENC_SW, INPUT);
  
  // Read initial state
  lastState = (digitalRead(ENC_A) << 1) | digitalRead(ENC_B);
  
  attachInterrupt(digitalPinToInterrupt(ENC_A), updateEncoder, CHANGE);
  attachInterrupt(digitalPinToInterrupt(ENC_B), updateEncoder, CHANGE);
  
  Serial.println("Rotary Encoder - Heavy Debounce");
  Serial.println("Position: 0");
}

void loop() {
  static int lastPos = 0;
  static int lastSW = 1;
  static unsigned long lastDebounceTime = 0;
  
  int currentSW = digitalRead(ENC_SW);
  
  if (encoderPos != lastPos) {
    // Additional software debounce in main loop
    if (millis() - lastDebounceTime > 50) {
      int delta = encoderPos - lastPos;
      Serial.print("Position: ");
      Serial.print(encoderPos);
      Serial.print(" (");
      Serial.print(delta > 0 ? "+" : "");
      Serial.print(delta);
      Serial.println(")");
      lastPos = encoderPos;
      lastDebounceTime = millis();
    } else {
      // Reject the noisy change
      encoderPos = lastPos;
    }
  }
  
  if (currentSW != lastSW) {
    if (currentSW == 0) {
      Serial.println("Button PRESSED");
    } else {
      Serial.println("Button RELEASED");
    }
    lastSW = currentSW;
  }
}

void updateEncoder() {
  static unsigned long lastInterruptTime = 0;
  
  // Aggressive debounce: 20ms
  unsigned long interruptTime = millis();
  if (interruptTime - lastInterruptTime < 20) {
    return;
  }
  lastInterruptTime = interruptTime;
  
  int currentState = (digitalRead(ENC_A) << 1) | digitalRead(ENC_B);
  
  // Simple state transition detection
  int transition = (lastState << 2) | currentState;
  
  // CW: 0b0001, 0b0111, 0b1110, 0b1000
  // CCW: 0b0010, 0b1011, 0b1101, 0b0100
  
  if (transition == 0b0001 || transition == 0b0111 || transition == 0b1110 || transition == 0b1000) {
    encoderPos++;
  } else if (transition == 0b0010 || transition == 0b1011 || transition == 0b1101 || transition == 0b0100) {
    encoderPos--;
  }
  
  lastState = currentState;
}