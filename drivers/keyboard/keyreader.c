#include <kernel/keyboard/keyreader.h>
#include <kernel/vga/vga.h>

int shift_pressed = 0;
int caps_on = 0;

char scancode_to_char(unsigned char scancode) {
    if (scancode == 0x2A || scancode == 0x36) {
        shift_pressed = 1;
        return 0;
    }
    if (scancode == 0x3A){
        if (caps_on==0) caps_on=1;
        else caps_on=0;
    }
    if (scancode == 0xAA || scancode == 0xB6) {
        shift_pressed = 0;
        return 0;
    }
    char buf[2] = {scancode, 0};
    
    char c = 0;
    if (scancode & 0x80) return 0;

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

        case 0x02: c=shift_pressed ? '!' : '1'; break; case 0x03: c=shift_pressed ? '@' : '2'; break;
        case 0x04: c=shift_pressed ? '#' : '3'; break; case 0x05: c=shift_pressed ? '$' : '4'; break;
        case 0x06: c=shift_pressed ? '%' : '5'; break; case 0x07: c=shift_pressed ? '^' : '6'; break;
        case 0x08: c=shift_pressed ? '&' : '7'; break; case 0x09: c=shift_pressed ? '*' : '8'; break;
        case 0x0A: c=shift_pressed ? '(' : '9'; break; case 0x0B: c=shift_pressed ? ')' : '0'; break;

        case 0x39: return ' ';
        case 0x1C: return '\n';
        default: return 0;
    }

    if (shift_pressed && c >= 'a' && c <= 'z') c -= 32;
    else if (caps_on) c -= 32;
    return c;
}
