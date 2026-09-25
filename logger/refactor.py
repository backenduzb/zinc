import re


CONVERTIBLE = (
    "println",
    "print_u64",
    "print_hex_u64",
    "print_hex_u32",
    "putc",
    "print_char",
    "print",
)

CALL_RE = re.compile(
    r"(?:crate::)?(?:drivers::)?serial::"
    r"(println|print_u64|print_hex_u64|print_hex_u32|putc|print_char|print)"
    r"\s*\("
)

BARE_RE = re.compile(
    r"(?<![_:a-zA-Z0-9])(println|print_u64|print_hex_u64|print_hex_u32|putc|print_char|print)"
    r"\s*\("
)
USE_SERIAL_RE = re.compile(
    r"use\s+(?:crate::)?(?:drivers::)?serial::\s*\{([^}]*)\}"
    r"|use\s+(?:crate::)?(?:drivers::)?serial::\s*([a-zA-Z0-9_]+)\s*;"
)

PLAIN_STR_RE = re.compile(r'^"(?:[^"\\]|\\.)*"$', re.DOTALL)
RAW_STR_RE = re.compile(r'^r(?P<hashes>#+)?"(?P<body>.*)"(?P=hashes)$', re.DOTALL)


class LogRefactorer:
    def __init__(self, filepath):
        self.filepath = filepath
        self.rel_path = filepath.lower().replace("\\", "/")

    def detect_macro(self, fmt_str):
        s = fmt_str.upper()
        p = self.rel_path

        if "CR3" in s:
            return "log_cr3"
        if "TSS" in s or "RSP0" in s:
            return "log_tss"
        if "SYSCALL" in s:
            return "log_syscall"
        if "SLEEP" in s:
            return "log_sleep"
        if (
            "VFS" in s
            or "OPEN" in s
            or "READ" in s
            or "WRITE" in s
            or "CLOSE" in s
            or "DIR" in s
            or "INODE" in s
            or "FILE" in s
        ):
            return "log_vfs"
        if "EXEC" in s or "REAP" in s or "ELF" in s:
            return "log_exec"
        if "RING3" in s or "RING 3" in s or "USER" in s:
            return "log_ring3"
        if "SCHED" in s or "TICK" in s or "YIELD" in s or "IDLE" in s:
            return "log_sched"
        if "SMP" in s or "CPU" in s or "APIC" in s or "LAPIC" in s:
            return "log_smp"
        if "KBD" in s or "SCANCODE" in s:
            return "log_kbd"
        if "BOOT" in s or "LIMINE" in s or "HHDM" in s or "RSDP" in s:
            return "log_boot"
        if "TEST" in s or "PASS" in s or "FAIL" in s:
            return "log_test"
        if "PANIC" in s:
            return "log_boot"
        if "HEAP" in s or "PHYS" in s or "VIRT" in s or "PAGE" in s or "MMAP" in s:
            return "log_boot"

        if "task/scheduler" in p or "sched" in p:
            return "log_sched"
        if "syscall" in p:
            return "log_syscall"
        if "sleep" in p:
            return "log_sleep"
        if "filesystem" in p or "vfs" in p or "inode" in p or "ext2" in p:
            return "log_vfs"
        if "exec" in p or "elf" in p:
            return "log_exec"
        if "smp" in p or "lapic" in p or "acpi" in p or "percpu" in p:
            return "log_smp"
        if "arch/x86_64/cpu" in p or "/cpu" in p or "arch/mod" in p:
            return "log_smp"
        if "keyboard" in p or "input" in p:
            return "log_kbd"
        if "tss" in p or "gdt" in p:
            return "log_tss"
        if "memory" in p or "paging" in p or "heap" in p:
            return "log_boot"
        if "panic" in p:
            return "log_boot"
        if "clock" in p or "timer" in p or "pit" in p:
            return "log_boot"
        if "test" in p:
            return "log_test"

        return "log_boot"

    @staticmethod
    def _escape_for_format(text):
        return text.replace("{", "{{").replace("}", "}}")

    def _literal_to_fmt(self, arg):
        a = arg.strip()
        if not a:
            return None, False
        if a.startswith('b"') or a.startswith("b'"):
            a = a[1:]
        m = PLAIN_STR_RE.match(a)
        if m:
            inner = a[1:-1]  
            return self._escape_for_format(inner), True
        m = RAW_STR_RE.match(a)
        if m:
            body = m.group("body")
            body = body.replace("\\", "\\\\").replace('"', '\\"')
            body = body.replace("\r\n", "\\n").replace("\n", "\\n").replace("\r", "\\n")
            return self._escape_for_format(body), True
        return None, False

    def convert_one(self, call_type, arg):
        arg = arg.strip()
        if call_type == "println":
            if not arg:
                return "", [], True
            lit, ok = self._literal_to_fmt(arg)
            if ok:
                return lit, [], True
            return "{}", [arg], True
        if call_type == "print":
            if not arg:
                return "", [], False
            lit, ok = self._literal_to_fmt(arg)
            if ok:
                return lit, [], False
            return "{}", [arg], False
        if call_type == "print_u64":
            return "{}", [arg] if arg else [], False
        if call_type == "print_hex_u64":
            return "{:016X}", [arg] if arg else [], False
        if call_type == "print_hex_u32":
            return "{:08X}", [arg] if arg else [], False
        if call_type == "putc":
            if not arg:
                return "", [], False
            return "{}", [f"({arg}) as char"], False
        if call_type == "print_char":
            if not arg:
                return "", [], False
            return "{}", [arg], False
        return "", [], False

    def _find_calls(self, content):
        bare_names = self._bare_imported_names(content)

        matches = []
        for m in CALL_RE.finditer(content):
            matches.append((m.start(), m.end(), m.group(1), True))
        if bare_names:
            for m in BARE_RE.finditer(content):
                name = m.group(1)
                if name not in bare_names:
                    continue
                prefix = content[max(0, m.start(1) - 2):m.start(1)]
                if prefix == "::":
                    continue
                after = content[m.end():m.end() + 1]
                if after == "!":
                    continue
                matches.append((m.start(1), m.end(), name, False))
        matches.sort()

        calls = []
        code_mask = self._code_mask(content)

        for start, arg_start, call_type, _qualified in matches:
            if not code_mask[start]:
                continue 
            parsed = self._parse_parens(content, arg_start)
            if parsed is None:
                continue
            arg_str, close_idx = parsed
            end_idx = close_idx + 1
            j = end_idx
            while j < len(content) and content[j] in (" ", "\t", "\r", "\n"):
                j += 1
            has_semi = j < len(content) and content[j] == ";"
            if has_semi:
                end_idx = j + 1
            calls.append(
                {
                    "type": call_type,
                    "arg": arg_str,
                    "start": start,
                    "end": end_idx,
                    "has_semicolon": has_semi,
                }
            )
        calls.sort(key=lambda c: (c["start"], c["end"]))
        filtered = []
        last_end = -1
        for c in calls:
            if c["start"] < last_end:
                continue
            filtered.append(c)
            last_end = c["end"]
        return filtered

    @staticmethod
    def _bare_imported_names(content):
        names = set()
        for m in USE_SERIAL_RE.finditer(content):
            braced, single = m.group(1), m.group(2)
            if braced:
                for part in braced.split(","):
                    part = part.strip().split(" as ")[0].strip()
                    if part in CONVERTIBLE:
                        names.add(part)
            elif single and single in CONVERTIBLE:
                names.add(single)
        return names

    @staticmethod
    def _is_char_start(content, i):
        n = len(content)
        if i + 1 >= n:
            return False
        nxt = content[i + 1]
        if nxt == "\\":
            j = i + 2
            while j < n and content[j] not in ("'", "\n"):
                j += 1
            return j < n and content[j] == "'"
        if i + 2 < n and content[i + 2] == "'":
            return True
        return False

    @staticmethod
    def _code_mask(content):
        n = len(content)
        mask = [True] * n
        i = 0
        state = "code"  
        raw_hashes = ""
        block_depth = 0
        while i < n:
            c = content[i]
            nxt = content[i + 1] if i + 1 < n else ""
            if state == "code":
                if c == "/" and nxt == "/":
                    state = "line_comment"
                    mask[i] = mask[i + 1] = False
                    i += 2
                    continue
                if c == "/" and nxt == "*":
                    state = "block_comment"
                    block_depth = 1
                    mask[i] = mask[i + 1] = False
                    i += 2
                    continue
                if (
                    c == "r"
                    and nxt in ('"', "#")
                    and (i == 0 or not (content[i - 1].isalnum() or content[i - 1] == "_"))
                ):
                    j = i + 1
                    hashes = ""
                    while j < n and content[j] == "#":
                        hashes += "#"
                        j += 1
                    if j < n and content[j] == '"':
                        state = "raw"
                        raw_hashes = hashes
                        for k in range(i, j + 1):
                            mask[k] = False
                        i = j + 1
                        continue
                if c == '"':
                    state = "string"
                    mask[i] = False
                    i += 1
                    continue
                if c == "'":
                    if LogRefactorer._is_char_start(content, i):
                        state = "char"
                        mask[i] = False
                    i += 1
                    continue
                i += 1
            elif state == "line_comment":
                mask[i] = False
                if c == "\n":
                    state = "code"
                i += 1
            elif state == "block_comment":
                mask[i] = False
                if c == "/" and nxt == "*":
                    block_depth += 1
                    mask[i + 1] = False
                    i += 2
                    continue
                if c == "*" and nxt == "/":
                    block_depth -= 1
                    mask[i + 1] = False
                    i += 2
                    if block_depth <= 0:
                        state = "code"
                    continue
                i += 1
            elif state == "string":
                mask[i] = False
                if c == "\\":
                    if i + 1 < n:
                        mask[i + 1] = False
                        i += 2
                    else:
                        i += 1
                    continue
                if c == '"':
                    state = "code"
                i += 1
            elif state == "char":
                mask[i] = False
                if c == "\\":
                    if i + 1 < n:
                        mask[i + 1] = False
                        i += 2
                    else:
                        i += 1
                    continue
                if c == "'":
                    state = "code"
                i += 1
            elif state == "raw":
                mask[i] = False
                if c == '"':
                    j = i + 1
                    ok = True
                    for _ in raw_hashes:
                        if j < n and content[j] == "#":
                            mask[j] = False
                            j += 1
                        else:
                            ok = False
                            break
                    if ok:
                        state = "code"
                        i = j
                        continue
                i += 1
        return mask

    @staticmethod
    def _parse_parens(content, arg_start):
        n = len(content)
        depth = 1
        i = arg_start
        state = "code"
        raw_hashes = ""
        while i < n and depth > 0:
            c = content[i]
            nxt = content[i + 1] if i + 1 < n else ""
            if state == "code":
                if c == "/" and nxt == "/":
                    state = "line_comment"
                    i += 2
                    continue
                if c == "/" and nxt == "*":
                    state = "block_comment"
                    i += 2
                    continue
                if (
                    c == "r"
                    and nxt in ('"', "#")
                    and (i == 0 or not (content[i - 1].isalnum() or content[i - 1] == "_"))
                ):
                    j = i + 1
                    hashes = ""
                    while j < n and content[j] == "#":
                        hashes += "#"
                        j += 1
                    if j < n and content[j] == '"':
                        state = "raw"
                        raw_hashes = hashes
                        i = j + 1
                        continue
                if c == '"':
                    state = "string"
                    i += 1
                    continue
                if c == "'":
                    if LogRefactorer._is_char_start(content, i):
                        state = "char"
                    i += 1
                    continue
                if c == "(":
                    depth += 1
                elif c == ")":
                    depth -= 1
                    if depth == 0:
                        return content[arg_start:i], i
                i += 1
            elif state == "line_comment":
                if c == "\n":
                    state = "code"
                i += 1
            elif state == "block_comment":
                if c == "*" and nxt == "/":
                    state = "code"
                    i += 2
                    continue
                i += 1
            elif state == "string":
                if c == "\\":
                    i += 2
                    continue
                if c == '"':
                    state = "code"
                i += 1
            elif state == "char":
                if c == "\\":
                    i += 2
                    continue
                if c == "'":
                    state = "code"
                i += 1
            elif state == "raw":
                if c == '"':
                    j = i + 1
                    ok = True
                    for _ in raw_hashes:
                        if j < n and content[j] == "#":
                            j += 1
                        else:
                            ok = False
                            break
                    if ok:
                        state = "code"
                        i = j
                        continue
                i += 1
        return None

    def process_content(self, content):
        calls = self._find_calls(content)
        if not calls:
            return content, 0

        new_content, replaced = self._rebuild(content, calls)
        return new_content, replaced

    def _rebuild(self, content, calls):
        out = []
        pos = 0
        replaced = 0
        for call in calls:
            fmt, args, newline = self.convert_one(call["type"], call["arg"])
            args = [a for a in args if a.strip()]
            if fmt == "" and not args and call["type"] != "println":
                continue
            macro_base = self.detect_macro(fmt)
            macro_name = macro_base if newline else macro_base + "_raw"
            if args:
                macro_body = f'crate::{macro_name}!("{fmt}", {", ".join(args)})'
            else:
                macro_body = f'crate::{macro_name}!("{fmt}")'
            if call["has_semicolon"]:
                macro_code = macro_body + ";"
            else:
                macro_code = macro_body
            out.append(content[pos:call["start"]])
            out.append(macro_code)
            pos = call["end"]
            replaced += 1
        out.append(content[pos:])
        return "".join(out), replaced
