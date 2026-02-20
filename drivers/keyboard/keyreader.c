#include <kernel/keyboard/keyreader.h>

char scancode_to_char(unsigned char scancode) {

    if (scancode & 0x80) return 0;

    // switch(scancode) {
    //     case 0x02: return shift_pressed ? '!' : '1';
    //     case 0x03: return shift_pressed ? '@' : '2';
    //     case 0x04: return shift_pressed ? '#' : '3';
    //     case 0x05: return shift_pressed ? '$' : '4';
    //     case 0x06: return shift_pressed ? '%' : '5';
    //     case 0x07: return shift_pressed ? '^' : '6';
    //     case 0x08: return shift_pressed ? '&' : '7';
    //     case 0x09: return shift_pressed ? '*' : '8';
    //     case 0x0A: return shift_pressed ? '(' : '9';
    //     case 0x0B: return shift_pressed ? ')' : '0';
    // }

    char c = 0;

    switch(scancode) {
        case 0x1E: c='a'; break; case 0x30: c='b'; break;
        case 0x2E: c='c'; break; case 0x20: c='d'; break;
        case 0x12: c='e'; break; case 0x21: c='f'; break;
        case 0x22: c='g'; break; case 0x23: c='h'; break;
        case 0x17: c='i'; break; case 0x24: c='j'; break;
        case 0x25: c='k'; break; case 0x26: c='l'; break;
        case 0x32: c='m'; break; case 0x31: c='n'; break;
        case 0x18: c='o'; break; case 0x19: c='p'; break;
        case 0x10: c='q'; break; case 0x13: c='r'; break;
        case 0x1F: c='s'; break; case 0x14: c='t'; break;
        case 0x16: c='u'; break; case 0x2F: c='v'; break;
        case 0x11: c='w'; break; case 0x2D: c='x'; break;
        case 0x15: c='y'; break; case 0x2C: c='z'; break;

        case 0x39: return ' ';
        case 0x1C: return '\n';
        case 0x0E: return 8;
        default: return 0;
    }

    return c;
}