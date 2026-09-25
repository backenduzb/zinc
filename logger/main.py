import os
import sys

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from refactor import LogRefactorer

SKIP_FILES = {"log.rs"}


def parse_args(argv):
    target_dir = None
    dry_run = False
    check = False
    for a in argv[1:]:
        if a == "--dry-run":
            dry_run = True
        elif a == "--check":
            check = True
        elif a.startswith("-"):
            print(f"Noma'lum flag: {a}")
            print("sample: python main.py [target_dir] [--dry-run] [--check]")
            sys.exit(2)
        elif target_dir is None:
            target_dir = a
        else:
            print("sample: python main.py [target_dir] [--dry-run] [--check]")
            sys.exit(2)
    return target_dir, dry_run, check


def collect_rs_files(target_dir):
    out = []
    for root, _, files in os.walk(target_dir):
        for file in files:
            if not file.endswith(".rs"):
                continue
            if file in SKIP_FILES:
                continue
            if file.endswith(".working"):
                continue
            out.append(os.path.join(root, file))
    return sorted(out)


def run():
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    kernel_src = os.path.join(base_dir, "kernel", "src")

    target_arg, dry_run, check = parse_args(sys.argv)
    target_dir = target_arg if target_arg else kernel_src

    if not os.path.exists(target_dir):
        print(f"[ERR]: Target folder not found: {target_dir}")
        sys.exit(1)

    print("=== ZINC LOG REFACTOR ===")
    print(f"Searching space: {target_dir}")
    if dry_run:
        print("(dry-run: not changing files)")
    print()

    total_files = 0
    total_calls_replaced = 0
    errors = 0

    for filepath in collect_rs_files(target_dir):
        try:
            with open(filepath, "r", encoding="utf-8") as f:
                content = f.read()
        except Exception as e:
            print(f"  [ERR] {filepath}: can't readed: {e}")
            errors += 1
            continue

        try:
            refactorer = LogRefactorer(filepath)
            new_content, replaced_count = refactorer.process_content(content)
        except Exception as e:
            print(f"  [ERR] {filepath}: parse error: {e}")
            errors += 1
            continue

        if replaced_count > 0 and new_content != content:
            rel_p = os.path.relpath(filepath, base_dir)
            if dry_run or check:
                print(f"  [DRY] {rel_p} -> {replaced_count} serial calls chaned")
            else:
                try:
                    with open(filepath, "w", encoding="utf-8") as f:
                        f.write(new_content)
                except Exception as e:
                    print(f"  [ERR] {rel_p}: not writed: {e}")
                    errors += 1
                    continue
                print(f"  [OK] {rel_p} -> {replaced_count} serial call changed!")
            total_files += 1
            total_calls_replaced += replaced_count

    print()
    print("=== FINISHED ===")
    print(f"Changed files count: {total_files}")
    print(f"Changed lines: {total_calls_replaced}")
    if errors:
        print(f"Errors: {errors}")
        sys.exit(1)
    if check and total_files > 0:
        sys.exit(1)


if __name__ == "__main__":
    run()
