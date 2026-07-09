# 修复 PopupMenu::new() 私有构造函数访问

## 问题

`gen_menu_bar_with_children_bind` 生成的递归辅助函数 `__rml_popup_item_0` 使用 `rml_ui::PopupMenu::new()` 作为 `std::mem::replace` 的占位值，但 `PopupMenu::new()` 是 `pub(crate)` 私有构造函数，导致编译失败。

## 根因

生成的函数签名为 `fn(&mut PopupMenu, ...)`，需要通过 `std::mem::replace` 替换 `&mut` 引用指向的值。但 PopupMenu 没有公开的默认构造函数。

## 解决方案

将递归辅助函数从 `&mut PopupMenu` 模式改为 `PopupMenu -> PopupMenu` 模式（值传递，返回值链）。这与 gpui-component 的 API 设计一致：`item(self)` 和 `submenu(self, ...)` 都接收 `self` 并返回 `Self`。

### 变更前后对比

**变更前（生成代码）：**
```rust
fn __rml_popup_item_0(menu: &mut rml_ui::PopupMenu, m: &MenuViewModel, ...) {
    if __rml_children.is_empty() {
        *menu = std::mem::replace(menu, rml_ui::PopupMenu::new()).item(item);
    } else {
        *menu = std::mem::replace(menu, rml_ui::PopupMenu::new())
            .submenu(label, window, cx, move |mut submenu, window, cx| {
                for __rml_c in &__rml_children {
                    __rml_popup_item_0(&mut submenu, __rml_c, window, cx);
                }
                submenu
            });
    }
}
```

**变更后（生成代码）：**
```rust
fn __rml_popup_item_0(menu: rml_ui::PopupMenu, m: &MenuViewModel, ...) -> rml_ui::PopupMenu {
    if __rml_children.is_empty() {
        menu.item(item)
    } else {
        menu.submenu(label, window, cx, move |submenu, window, cx| {
            let mut submenu = rml_ui::configure_menu_bar_popup(submenu);
            let mut submenu = submenu;
            for __rml_c in &__rml_children {
                submenu = __rml_popup_item_0(submenu, __rml_c, window, cx);
            }
            submenu
        })
    }
}
```

## 修改文件

### 1. `crates/engine/src/compiler/components/menu/menu_bar.rs`

修改 `gen_menu_bar_with_children_bind` 函数中的 `fn_def` 和 `top_code` 模板字符串。

**fn_def 变更点：**
- 函数签名：`menu: &mut rml_ui::PopupMenu` → `menu: rml_ui::PopupMenu`，添加 `-> rml_ui::PopupMenu`
- 叶子分支：`*menu = std::mem::replace(menu, rml_ui::PopupMenu::new()).item(item)` → `menu.item(item)`
- 子菜单分支：`*menu = std::mem::replace(menu, rml_ui::PopupMenu::new()).submenu(...)` → `menu.submenu(...)`
- submenu 闭包：`move |mut submenu, window, cx|` → `move |submenu, window, cx|`，添加 `let mut submenu = submenu;`，递归调用改为 `submenu = __rml_popup_item_N(submenu, ...)`

**top_code 变更点：**
- dropdown_menu 闭包：`move |mut menu, window, cx|` → `move |menu, window, cx|`，添加 `let mut menu = menu;`
- 递归调用：`__rml_popup_item_N(&mut menu, ...)` → `menu = __rml_popup_item_N(menu, ...)`
- 闭包末尾 `menu` 返回值保持不变

### 2. 更新 `menu_bar.rs` 中的单元测试

现有测试断言基于旧的 `macro_rules!` 模式，需更新为当前函数模式的断言：
- `children_bind_generates_recursive_macro`：更新为检查函数定义而非宏
- `children_bind_label_field_access`：更新变量名从 `__rml_child` 到 `__rml_children`

## 验证

1. `cargo build -p rust-rml-demo` — 编译通过
2. `cargo test -p rml-engine` — 所有测试通过
3. 运行 demo 应用验证菜单功能正常
