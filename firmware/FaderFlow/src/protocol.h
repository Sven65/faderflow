#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <Arduino.h>

#define MAGIC_STRING "FADERFLOW"
#define UUID_SIZE 16

// Command bytes
#define CMD_HANDSHAKE_REQUEST 0x01
#define CMD_HANDSHAKE_ACK 0x02
#define CMD_HANDSHAKE_RESPONSE 0x03
#define CMD_ECHO_UUID 0x04
#define CMD_DISPLAY_UPDATE_APP_NAME 0x05
#define CMD_DISPLAY_UPDATE_APP_VOLUME 0x06
#define CMD_DISPLAY_UPDATE_ICON 0x07

#define CMD_FADER_UPDATE 0x10


typedef struct {
    uint8_t cmd;
    char magic[10];        // "FADERFLOW\0"
    uint8_t device_type;   // 0x01 for volume controller
    uint8_t uuid[UUID_SIZE];
    uint8_t version_major;
    uint8_t version_minor;
} __attribute__((packed)) HandshakeResponse;

typedef struct {
    uint8_t cmd;
    uint8_t channel;
    uint8_t position;
} __attribute__((packed)) FaderMessage;

typedef struct {
    uint8_t cmd;        // CMD_SET_APP
    uint8_t channel;
    char name[64];
    // Followed by: char name[nameLen]
} __attribute__((packed)) DisplayUpdateAppCommand;

typedef struct {
    uint8_t cmd;        // CMD_SET_VOLUME
    uint8_t channel;
    uint8_t volume;     // 0-100
} __attribute__((packed)) DisplayUpdateVolumeCommand;

typedef struct {
    uint8_t cmd;        // CMD_SET_ICON
    uint8_t channel;
    // Followed by: uint8_t iconData[8192]
} __attribute__((packed)) DisplayUpdateIconCommand;

#endif