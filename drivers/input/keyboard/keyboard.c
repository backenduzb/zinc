#include <ui/terminal.h>
#include <input/keyboard.h>
#include <io.h>
#include <pic.h>
#include <stdint.h>

#define KBD_DATA_PORT 0x60

static volatile uint8_t shift_down = 0;
static volatile uint8_t caps_down = 0;

static const char scancode_map[128] = {
    0,  27, '1', '2', '3', '4', '5', '6', '7', '8', '9', '0', '-', '=', '\b',  0,
    'q','w','e','r','t','y','u','i','o','p','[',']','\n',0,  'a','s',
    'd','f','g','h','j','k','l',';','\'','`',0, '\\','z','x','c','v',
    'b','n','m',',','.','/',0,  '*',0,  ' ',0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
};

static const char scancode_shift_map[128] = {
    0,  27, '!', '@', '#', '$', '%', '^', '&', '*', '(', ')', '_', '+', '\b',  0,
    'Q','W','E','R','T','Y','U','I','O','P','{','}','\n',0,  'A','S',
    'D','F','G','H','J','K','L',':','"','~',0,  '|','Z','X','C','V',
    'B','N','M','<','>','?',0,  '*',0,  ' ',0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,
    0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0,  0
};

static char scancode_to_ascii(uint8_t scancode) {
    if (scancode >= 128) {
        return 0;
    }
    char ch;
    if (shift_down) ch = scancode_shift_map[scancode];
    else ch = scancode_map[scancode];

    if (caps_down && ch >= 'a' && ch <= 'z') ch -= 32;
    else if (caps_down && ch >= 'A' && ch <= 'Z') ch +=32;
    return ch;
}

void keyboard_init(void) {
    shift_down = 0;
}

void keyboard_handler(void) {
    uint8_t scancode = inb(KBD_DATA_PORT);

    if (scancode == 0xE0 || scancode == 0xE1) {
        pic_send_eoi(1);
        return;
    }

    if (scancode & 0x80) {
        uint8_t key = scancode & 0x7F;
        if (key == 0x2A || key == 0x36) {
            shift_down = 0;
        }
        pic_send_eoi(1);
        return;
    }
    if (scancode == 0x2A || scancode == 0x36) {
        shift_down = 1;
        pic_send_eoi(1);
        return;
    }
    
    if (scancode == 0x3A) {
        caps_down = !caps_down;
        pic_send_eoi(1);
        return;
    }

    char ch = scancode_to_ascii(scancode);
    termwrite(ch, 0x00FFFFFF);


    pic_send_eoi(1);
}
