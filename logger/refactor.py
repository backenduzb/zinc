import re
import os

class LogRefactorer:
    def __init__(self, filepath):
        self.filepath = filepath
        self.rel_path = filepath.lower().replace('\\', '/')

    def detect_macro(self, fmt_str):
        s = fmt_str.upper()
        p = self.rel_path

        if "CR3" in s:
            return "log_cr3"
        if "TSS" in s or "RSP0" in s:
            return "log_tss"
        if "SYSCALL" in s:
            return "log_syscall"
        if "VFS" in s or "OPEN" in s or "READ" in s or "WRITE" in s:
            return "log_vsf"
        if "EXEC" in s:
            return "log_exec"
        if "RING3" in s or "RING 3" in s:
            return "log_ring3"
        if "SCHED" in s or "TICK" in s:
            return "log_sched"
        if "SMP" in s or "CPU" in s:
            return "log_smp"
        if "KBD" in s or "SCANCODE" in s:
            return "log_kbd"

        if "task/scheduler" in p or "sched" in p:
            return "log_sched"
        if "syscall" in p:
            return "log_syscall"
        if "filesystem" in p or "vfs" in p:
            return "log_vsf"
        if "exec" in p:
            return "log_exec"
        if "arch/x86_64/smp" in p:
            return "log_smp"
        if "keyboard" in p or "input" in p:
            return "log_kbd"
        if "tss" in p:
            return "log_tss"

        return "log_ring3"

    def parse_call_arg(self, call_type, arg):
        arg = arg.strip()
        
        is_string_literal = bool(re.match(r'^(?:r#*|b)?".*"#*$', arg, re.DOTALL))

        if call_type in ("println", "print"):
            if is_string_literal:
                clean_str = re.sub(r'^(?:r#*|b)?"|"#*$', '', arg)
                return clean_str, []
            else:
                return "{}", [arg]
        elif call_type == "print_u64":
            return "{}", [arg]
        elif call_type == "print_hex_u64":
            return "0x{:x}", [arg]

        return "", []

    def process_content(self, content):
        target_pattern = re.compile(
            r'(?:crate::)?(?:drivers::)?serial::(println|print_u64|print_hex_u64|print)\s*\('
        )

        calls = []
        pos = 0

        while pos < len(content):
            match = target_pattern.search(content, pos)
            if not match:
                break

            call_type = match.group(1)
            start_idx = match.start()
            arg_start = match.end()

            depth = 1
            i = arg_start
            in_string = False
            string_char = None

            while i < len(content) and depth > 0:
                ch = content[i]
                if in_string:
                    if ch == string_char and content[i-1] != '\\':
                        in_string = False
                else:
                    if ch in ('"', "'"):
                        in_string = True
                        string_char = ch
                    elif ch == '(':
                        depth += 1
                    elif ch == ')':
                        depth -= 1
                i += 1

            if depth == 0:
                arg_str = content[arg_start:i-1]
                end_idx = i
                
                while end_idx < len(content) and content[end_idx] in (' ', '\t', '\r', '\n'):
                    end_idx += 1
                if end_idx < len(content) and content[end_idx] == ';':
                    end_idx += 1

                calls.append({
                    'type': call_type,
                    'arg': arg_str,
                    'start': start_idx,
                    'end': end_idx
                })
                pos = end_idx
            else:
                pos = arg_start

        if not calls:
            return content, 0

        groups = []
        curr_group = [calls[0]]

        for c in calls[1:]:
            prev = curr_group[-1]
            between = content[prev['end']:c['start']]
            if between.strip() == "":
                curr_group.append(c)
            else:
                groups.append(curr_group)
                curr_group = [c]
        groups.append(curr_group)

        new_content = list(content)
        total_replaced = 0

        for group in reversed(groups):
            fmt_parts = []
            args = []

            for call in group:
                fmt, arg_list = self.parse_call_arg(call['type'], call['arg'])
                fmt_parts.append(fmt)
                args.extend(arg_list)

            fmt_string = "".join(fmt_parts)
            macro_name = self.detect_macro(fmt_string)

            if args:
                args_fmt = ", ".join(args)
                macro_code = f'crate::{macro_name}!("{fmt_string}", {args_fmt});'
            else:
                macro_code = f'crate::{macro_name}!("{fmt_string}");'

            g_start = group[0]['start']
            g_end = group[-1]['end']

            new_content[g_start:g_end] = list(macro_code)
            total_replaced += len(group)

        return "".join(new_content), total_replaced