#!/usr/bin/env python3
"""RML 样式属性归一化迁移脚本

将旧版 Tailwind 式样式属性批量替换为归一化后的新属性。
映射表对齐 .trae/skills/rml-component/migration-style-normalization.md。

仅处理旧属性的标准写法 `name=""`（空字符串值，RML 布尔标志约定）。
不处理裸属性形式 `name`（无 `=""`），因为 `flex-wrap` 等属性名在新规范
中仍存在，裸匹配会误伤新属性 `flex-wrap="wrap"` 的属性名部分。

用法：
    python scripts/migrate_style_attrs.py <path>              # dry-run 预览
    python scripts/migrate_style_attrs.py <path> --write      # 实际写入（默认备份 .bak）
    python scripts/migrate_style_attrs.py <path> --write --no-backup
"""

from __future__ import annotations

import argparse
import re
import shutil
import sys
from pathlib import Path

# 静态属性映射（旧属性名 → 新属性串，值固定）
STATIC_MIGRATIONS: dict[str, str] = {
    "v-flex": 'display="flex" flex-direction="column"',
    "h-flex": 'display="flex" flex-direction="row"',
    "h-full": 'height="full"',
    "w-full": 'width="full"',
    "min-w-0": 'min-width="0"',
    "min-h-0": 'min-height="0"',
    "items-center": 'align-items="center"',
    "flex-wrap": 'flex-wrap="wrap"',
}

# 动态属性映射（旧 → 新，需按数值计算 px）
# (编译后的正则, 替换函数)
DYNAMIC_MIGRATIONS: list[tuple[re.Pattern, callable]] = [
    (re.compile(r'\bgap-(\d+)=""'), lambda m: f'gap="{int(m.group(1)) * 4}px"'),
    (re.compile(r'\bp-(\d+)=""'), lambda m: f'padding="{int(m.group(1)) * 4}px"'),
]


def migrate_content(content: str) -> tuple[str, int]:
    """对单个文件内容应用所有迁移规则，返回 (新内容, 替换次数)。"""
    total = 0
    # 先处理动态映射（gap-N, p-N），避免静态规则误伤
    for pattern, repl in DYNAMIC_MIGRATIONS:
        content, n = pattern.subn(repl, content)
        total += n
    # 再处理静态映射
    for old, new in STATIC_MIGRATIONS.items():
        pattern = re.compile(r'\b' + re.escape(old) + r'=""')
        content, n = pattern.subn(new, content)
        total += n
    return content, total


def find_rml_files(path: Path) -> list[Path]:
    """收集目标路径下的所有 .rml 文件。"""
    if path.is_file():
        return [path] if path.suffix == ".rml" else []
    if path.is_dir():
        return sorted(path.rglob("*.rml"))
    return []


def main() -> int:
    parser = argparse.ArgumentParser(
        description="RML 样式属性归一化迁移脚本",
        usage="%(prog)s <path> [--write] [--no-backup]",
    )
    parser.add_argument("path", type=Path, help="目标 .rml 文件或目录")
    parser.add_argument("--write", action="store_true", help="实际写入文件（默认 dry-run 预览）")
    parser.add_argument("--no-backup", action="store_true", help="写入时不创建 .bak 备份")
    args = parser.parse_args()

    if not args.path.exists():
        print(f"错误：路径不存在：{args.path}", file=sys.stderr)
        return 2

    files = find_rml_files(args.path)
    if not files:
        print(f"未找到 .rml 文件：{args.path}", file=sys.stderr)
        return 1

    # 收集变更
    changes: list[tuple[Path, str, str, int]] = []  # (file, original, migrated, count)
    for f in files:
        original = f.read_text(encoding="utf-8")
        migrated, count = migrate_content(original)
        if count > 0:
            changes.append((f, original, migrated, count))

    if not changes:
        print(f"无需迁移：{len(files)} 个 .rml 文件均无旧属性残留。")
        return 0

    # dry-run 预览
    if not args.write:
        total = 0
        for f, original, migrated, count in changes:
            total += count
            print(f"\n{f}  ({count} 处变更)")
            old_lines = original.splitlines()
            new_lines = migrated.splitlines()
            for i, (a, b) in enumerate(zip(old_lines, new_lines), 1):
                if a != b:
                    print(f"  L{i}: - {a.strip()}")
                    print(f"       + {b.strip()}")
        print(f"\n预览完成：{len(changes)} 个文件，{total} 处变更")
        print("使用 --write 实际写入。")
        return 0

    # 实际写入
    total = 0
    for f, original, migrated, count in changes:
        if not args.no_backup:
            backup = f.with_suffix(f.suffix + ".bak")
            shutil.copy2(f, backup)
        f.write_text(migrated, encoding="utf-8")
        total += count
        print(f"已写入 {f}  ({count} 处)")

    suffix = "" if args.no_backup else "（已备份 .bak）"
    print(f"\n迁移完成：{len(changes)} 个文件，{total} 处变更{suffix}。")
    return 0


if __name__ == "__main__":
    sys.exit(main())
