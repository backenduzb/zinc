#include <fb/fb.h>
#include <font/psf1.h>
#include <stdint.h>

#include <disk/ata.h>
#include <power.h>
#include <utils/string.h>

#include "terminal.h"

typedef void (*terminal_command_handler_t)(int argc, char **argv);

typedef struct {
    const char *name;
    const char *usage;
    terminal_command_handler_t handler;
} terminal_command_t;

static void command_help(int argc, char **argv);
static void command_clear(int argc, char **argv);
static void command_shutdown(int argc, char **argv);
static void command_reboot(int argc, char **argv);
static void command_write(int argc, char **argv);
static void command_read(int argc, char **argv);
static void command_hexdump(int argc, char **argv);
static void command_drawsector(int argc, char **argv);

static const terminal_command_t terminal_commands[] = {
    { "help", "help", command_help },
    { "clear", "clear", command_clear },
    { "mdown", "mdown", command_shutdown },
    { "reboot", "reboot", command_reboot },
    { "write", "write <sektor> [belgi]", command_write },
    { "read", "read <sektor>", command_read },
    { "hexdump", "hexdump <sektor>", command_hexdump },
    { "drawsector", "drawsector <sektor>", command_drawsector },
};

static int parse_u32(const char *text, uint32_t *value) {
    uint32_t base = 10;
    uint32_t result = 0;

    if (text == 0 || value == 0 || *text == '\0') {
        return 0;
    }

    if (text[0] == '0' && (text[1] == 'x' || text[1] == 'X')) {
        base = 16;
        text += 2;
    }

    if (*text == '\0') {
        return 0;
    }

    for (; *text; ++text) {
        uint32_t digit;

        if (*text >= '0' && *text <= '9') {
            digit = (uint32_t)(*text - '0');
        } else if (*text >= 'a' && *text <= 'f') {
            digit = (uint32_t)(*text - 'a' + 10);
        } else if (*text >= 'A' && *text <= 'F') {
            digit = (uint32_t)(*text - 'A' + 10);
        } else {
            return 0;
        }

        if (digit >= base) {
            return 0;
        }

        if (result > (0xFFFFFFFFu - digit) / base) {
            return 0;
        }

        result = result * base + digit;
    }

    *value = result;
    return 1;
}

static void print_usage(const char *usage) {
    terminal_writestring("\n[!] Ishlatish: ");
    terminal_writestring(usage);
    terminal_writestring("\n");
}

static void print_sector_dump(const uint8_t *buffer, uint32_t lba) {
    terminal_writestring("\n=== Sector ");
    terminal_writeint(lba);
    terminal_writestring(" (512 bayt) ===\n");

    for (uint32_t offset = 0; offset < ATA_SECTOR_SIZE; offset += 16) {
        terminal_writehex(offset, 4);
        terminal_writestring(": ");

        for (uint32_t i = 0; i < 16; ++i) {
            terminal_writehex(buffer[offset + i], 2);
            terminal_writestring(" ");
        }

        terminal_writestring(" |");

        for (uint32_t i = 0; i < 16; ++i) {
            uint8_t ch = buffer[offset + i];
            terminal_writestring_char((ch >= 32 && ch <= 126) ? (char)ch : '.');
        }

        terminal_writestring("|\n");
    }

    terminal_writestring("========================\n");
}

static void print_command_list(void) {
    terminal_writestring("\nAvailable commands:\n");
    for (uint32_t i = 0; i < (sizeof(terminal_commands) / sizeof(terminal_commands[0])); ++i) {
        terminal_writestring("  ");
        terminal_writestring(terminal_commands[i].name);
        terminal_writestring(" - ");
        terminal_writestring(terminal_commands[i].usage);
        terminal_writestring("\n");
    }
}

static int ensure_disk_ready(void) {
    static int probe_done;
    static int probe_ok;

    if (!probe_done) {
        probe_ok = (ata_init() == 0);
        probe_done = 1;
    }

    return probe_ok ? 0 : -1;
}

static int read_one_sector(uint32_t sector, uint8_t *buffer) {
    if (ensure_disk_ready() != 0) {
        terminal_writestring("\n[!] ATA disk topilmadi yoki tayyor emas\n");
        return -1;
    }

    if (ata_read_sectors(sector, 1, buffer) != 0) {
        terminal_writestring("\n[!] Diskdan o'qib bo'lmadi\n");
        return -1;
    }

    return 0;
}

static void command_help(int argc, char **argv) {
    (void)argc;
    (void)argv;
    print_command_list();
}

static void command_clear(int argc, char **argv) {
    (void)argc;
    (void)argv;
    terminal_clear();
}

static void command_shutdown(int argc, char **argv) {
    (void)argc;
    (void)argv;
    shutdown();
}

static void command_reboot(int argc, char **argv) {
    (void)argc;
    (void)argv;
    reboot();
}

static void command_write(int argc, char **argv) {
    uint8_t write_buf[ATA_SECTOR_SIZE];
    uint32_t sector = 0;
    uint8_t fill_char = 'A';

    if (argc < 2 || !parse_u32(argv[1], &sector)) {
        print_usage("write <sektor> [belgi]");
        return;
    }

    if (argc >= 3 && argv[2][0] != '\0') {
        fill_char = (uint8_t)argv[2][0];
    }

    for (uint32_t i = 0; i < ATA_SECTOR_SIZE; ++i) {
        write_buf[i] = fill_char;
    }

    terminal_writestring("\n[*] Sektor ");
    terminal_writeint(sector);
    terminal_writestring(" ga yozilmoqda...\n");

    if (ata_write_sectors(sector, 1, write_buf) != 0) {
        terminal_writestring("[!] Diskka yozib bo'lmadi\n");
        return;
    }

    terminal_writestring("[+] Yozish tugadi!\n");

    if (read_one_sector(sector, write_buf) == 0) {
        terminal_writestring("[*] Yozilgan ma'lumot tekshiruvi:\n");
        print_sector_dump(write_buf, sector);
    }
}

static void command_read(int argc, char **argv) {
    uint8_t buffer[ATA_SECTOR_SIZE];
    uint32_t sector = 0;

    if (argc < 2 || !parse_u32(argv[1], &sector)) {
        print_usage("read <sektor>");
        return;
    }

    terminal_writestring("\n[*] Sektor ");
    terminal_writeint(sector);
    terminal_writestring(" o'qilmoqda...\n");

    if (read_one_sector(sector, buffer) != 0) {
        return;
    }

    terminal_writestring("[+] O'qish tugadi!\n");
    print_sector_dump(buffer, sector);
}

static void command_hexdump(int argc, char **argv) {
    uint8_t buffer[ATA_SECTOR_SIZE];
    uint32_t sector = 0;

    if (argc < 2 || !parse_u32(argv[1], &sector)) {
        print_usage("hexdump <sektor>");
        return;
    }

    if (read_one_sector(sector, buffer) != 0) {
        return;
    }

    print_sector_dump(buffer, sector);
}

static void command_drawsector(int argc, char **argv) {
    uint8_t buffer[ATA_SECTOR_SIZE];
    uint32_t sector = 0;

    if (argc < 2 || !parse_u32(argv[1], &sector)) {
        print_usage("drawsector <sektor>");
        return;
    }

    if (read_one_sector(sector, buffer) != 0) {
        return;
    }

    terminal_clear();

    int x = 0;
    int y = 0;
    for (uint32_t i = 0; i < ATA_SECTOR_SIZE; ++i) {
        uint8_t ch = buffer[i];

        draw_char(x, y, (ch >= 32 && ch <= 126) ? (char)ch : '.', (ch >= 32 && ch <= 126) ? 0x00FFFFFF : 0x00000088);

        ++x;
        if (x >= 80) {
            x = 0;
            ++y;
            if (y >= 25) {
                break;
            }
        }
    }

    terminal_writestring("\n[+] Sektor framebufferda chizildi!\n");
}

int terminal_execute_command(char *command) {
    char *argv[10];
    int argc = 0;

    char *token = strtok(command, " ");
    while (token != NULL && argc < 10) {
        argv[argc++] = token;
        token = strtok(NULL, " ");
    }

    if (argc == 0) {
        return 0;
    }

    for (uint32_t i = 0; i < (sizeof(terminal_commands) / sizeof(terminal_commands[0])); ++i) {
        if (strcmp(argv[0], terminal_commands[i].name) == 0) {
            terminal_commands[i].handler(argc, argv);
            return 1;
        }
    }

    terminal_writestring("Unknown command: ");
    terminal_writestring(argv[0]);
    terminal_writestring("\n");
    return 0;
}