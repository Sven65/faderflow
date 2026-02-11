#ifndef PROTOCOL_H
#define PROTOCOL_H

#include <Arduino.h>

#define MAGIC_STRING "FADERFLOW"
#define UUID_SIZE 16

// Command bytes
#define CMD_HANDSHAKE_REQUEST 0x01
#define CMD_ECHO_UUID 0x02
#define CMD_FADER_UPDATE 0x10

typedef struct {
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

#endif