import os
import sys

sys.path.append(os.path.dirname(os.path.abspath(__file__)))

from refactor import LogRefactorer

def run():
    base_dir = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
    kernel_src = os.path.join(base_dir, "kernel", "src")

    target_dir = sys.argv[1] if len(sys.argv) > 1 else kernel_src

    if not os.path.exists(target_dir):
        print(f"Xato: Target papka topilmadi: {target_dir}")
        sys.exit(1)

    print(f"=== ZINC LOG REFACTOR ===")
    print(f"Qidirilayotgan joy: {target_dir}\n")

    total_files = 0
    total_calls_replaced = 0

    for root, _, files in os.walk(target_dir):
        for file in files:
            if file.endswith('.rs') and file != 'log.rs' and not file.endswith('.working'):
                filepath = os.path.join(root, file)

                with open(filepath, 'r', encoding='utf-8') as f:
                    content = f.read()

                refactorer = LogRefactorer(filepath)
                new_content, replaced_count = refactorer.process_content(content)

                if replaced_count > 0:
                    with open(filepath, 'w', encoding='utf-8') as f:
                        f.write(new_content)

                    rel_p = os.path.relpath(filepath, base_dir)
                    print(f"  [OK] {rel_p} -> {replaced_count} ta serial chaqiruv almashtirildi.")
                    total_files += 1
                    total_calls_replaced += replaced_count

    print("\n=== FINISHED ===")
    print(f"Changed files counf: {total_files}")
    print(f"Changed lines: {total_calls_replaced}")

if __name__ == "__main__":
    run()