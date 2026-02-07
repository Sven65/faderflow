# FaderFlow - Project TODO

## ✅ Phase 0: Foundation (DONE)
- [x] Basic Iced UI setup
- [x] Windows audio session enumeration
- [x] Volume control (bidirectional sync)
- [x] Mute control (bidirectional sync)
- [x] App icon extraction
- [x] Cross-platform architecture (trait-based backend)
- [x] COM callback system for external changes
- [x] Batch processing to prevent UI jitter

## 🔨 Phase 1: Complete Audio Backend
- [ ] **Device Management**
    - [ ] Enumerate all audio output devices
    - [ ] Track which device each session is using (read-only for now)
    - [ ] Display device name in UI
    - [ ] System-wide default device switching
    - [ ] Detect when devices are plugged/unplugged

- [ ] **Session Management**
    - [ ] Handle sessions appearing/disappearing smoothly
    - [ ] Persist session-to-channel mappings
    - [ ] Auto-assign new sessions to free channels
    - [ ] Handle duplicate app names (Chrome tab 1, 2, 3...)

- [ ] **UI Polish**
    - [ ] Better layout (grid view for channels)
    - [ ] Visual feedback for which device is active
    - [ ] Session search/filter
    - [ ] Master volume control
    - [ ] System tray integration
    - [ ] Minimize to tray

## 🔌 Phase 2: Hardware Protocol
- [ ] **Serial Communication**
    - [ ] Define protocol format (JSON? Binary? MessagePack?)
    - [ ] Implement serial port detection/connection
    - [ ] Send motor position commands
        - [ ] Protocol: `MOTOR <channel> <position_0-100>`
    - [ ] Send screen update commands
        - [ ] Protocol: `SCREEN <channel> <icon_data> <name> <volume%>`
    - [ ] Receive fader position updates
        - [ ] Protocol: `FADER <channel> <position_0-100>`
    - [ ] Receive knob rotation updates
        - [ ] Protocol: `KNOB <channel> <delta>`

- [ ] **Motor Sync**
    - [ ] When Windows volume changes → move motor
    - [ ] Smooth motor movements (acceleration curves)
    - [ ] Handle motor conflicts (user moving while motor moves)
    - [ ] Calibration routine on startup

- [ ] **Screen Updates**
    - [ ] Convert icons to format your displays need (resolution, color depth)
    - [ ] Optimize icon transfer (only send when changed)
    - [ ] Handle missing icons (default placeholder)
    - [ ] Show volume percentage and mute status on screen

## 📊 Phase 3: Profile System
- [ ] **Profile Management**
    - [ ] Create profile structure (JSON config file?)
    - [ ] Save profiles: `profiles/<device_name>.json`
    - [ ] Profile contents:
        - [ ] Channel assignments (which app on which fader)
        - [ ] Custom names/colors per channel
        - [ ] Master volume settings
    - [ ] Auto-switch profile when output device changes
    - [ ] UI for creating/editing/deleting profiles
    - [ ] Import/export profiles

- [ ] **Channel Mapping**
    - [ ] Assign specific apps to specific channels
    - [ ] Wildcard matching (e.g., "chrome*" → channel 1)
    - [ ] Priority system (if multiple apps match)
    - [ ] "Unmapped apps" overflow handling

## 🎛️ Phase 4: Advanced Features
- [ ] **Per-App Audio Routing** (HARD)
    - [ ] Research options:
        - [ ] Study SoundSwitch source code
        - [ ] Study AudioRouter source code
        - [ ] Investigate IPolicyConfig COM interface
        - [ ] Consider app restart approach
    - [ ] Choose implementation strategy
    - [ ] Implement device switching
    - [ ] Handle edge cases (apps that don't respond)

- [ ] **Hotkeys**
    - [ ] Global hotkey support
    - [ ] Toggle mute for specific apps
    - [ ] Quick profile switching
    - [ ] Push-to-talk style muting

- [ ] **Advanced UI**
    - [ ] Dark/light theme
    - [ ] Customizable colors per channel
    - [ ] Visualizers (audio level meters)
    - [ ] Mini mode (compact overlay)
    - [ ] Always-on-top option

## 🐧 Phase 5: Cross-Platform (Future)
- [ ] **Linux Support**
    - [ ] PulseAudio backend implementation
    - [ ] PipeWire backend (modern Linux)
    - [ ] Icon extraction for Linux
    - [ ] Test on various distros

- [ ] **macOS Support** (maybe?)
    - [ ] CoreAudio backend implementation
    - [ ] Icon extraction for macOS
    - [ ] Handle macOS sandboxing/permissions

## 🔧 Phase 6: Polish & Release
- [ ] **Testing**
    - [ ] Test with many simultaneous apps
    - [ ] Test device hotplug scenarios
    - [ ] Test with different audio hardware
    - [ ] Memory leak testing (COM cleanup)
    - [ ] Long-running stability test

- [ ] **Documentation**
    - [ ] User guide
    - [ ] Hardware build guide
    - [ ] Protocol documentation
    - [ ] Firmware examples for Arduino/ESP32
    - [ ] Troubleshooting guide

- [ ] **Distribution**
    - [ ] Windows installer
    - [ ] Auto-updater
    - [ ] GitHub releases
    - [ ] AUR package (for Arch Linux)

## 🎯 Current Sprint
**Focus: Complete Phase 1 - Audio Backend**

### This Week:
1. Finish device enumeration in UI
2. Add device picker dropdown (read-only for now)
3. Implement session persistence between app restarts
4. Add system tray icon

### Next Week:
1. Start hardware protocol design
2. Set up serial communication testing
3. Mock hardware for development (simulate with keyboard?)

---

## Notes & Ideas
- Consider using `egui` instead of `iced` if you need faster iteration?
- Look into Windows MIDI API for ultra-low-latency control?
- Could add OBS integration (control audio sources)?
- Discord/TeamSpeak integration for per-person volume?
- Game detection (auto-load "Gaming" profile)?
- Time-based profiles (work hours vs evening)?

---

## Hardware Waiting On:
- [ ] Motorized faders (ordered)
- [ ] Screens (TFT? OLED? e-ink?)
- [ ] Rotary encoders for knobs
- [ ] Microcontroller (Arduino? ESP32? Teensy?)
- [ ] Case/enclosure design

---

**Keep this updated as you go!** 🚀