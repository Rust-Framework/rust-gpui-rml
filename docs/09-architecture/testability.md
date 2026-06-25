# 9.5 可测试性设计

> **本节目标**：让 RML 代码可被高效测试——ViewModel 纯逻辑单测、组件快照测试、跨模块集成测试。

## 9.5.1 RML 测试的三层金字塔

```
                    ┌──────────┐
                    │  E2E     │  少量：完整用户流程
                    └──────────┘
                ┌───────────────┐
                │  集成测试       │  中量：跨模块协作
                └───────────────┘
        ┌───────────────────────────┐
        │  单元测试（ViewModel / Model）│  大量：纯逻辑
        └───────────────────────────┘
```

- **单元测试**：最快、最多，覆盖 ViewModel 的命令和计算属性
- **集成测试**：中速，覆盖视图 + Service 的协作
- **E2E**：最慢、最少，覆盖关键用户流程

## 9.5.2 让 ViewModel 可单测的关键

ViewModel 难测的根因是它依赖 `ViewContext`。可单测的 ViewModel 遵循两条规则：

1. **纯逻辑方法不依赖 `cx`**：计算、校验、派生数据的方法只读 `self`
2. **副作用方法依赖 trait 而非具体实现**：通过依赖注入替换

### 反例：不可测的 ViewModel

```rust
#[derive(Model)]
pub struct UserViewModel {
    pub user: User,
}

impl UserViewModel {
    #[command]
    pub fn save(&mut self, cx: &mut ViewContext<Self>) {
        let resp = reqwest::blocking::post("/users").json(&self.user).send().unwrap();
        if resp.status().is_success() {
            self.user.saved = true;
            cx.notify();
        }
    }
}
```

无法在无网络环境下测试，也无法验证失败分支。

### 正例：可测的 ViewModel

```rust
pub trait UserRepo: 'static {
    fn save(&self, user: &User) -> Task<Result<()>>;
}

#[derive(Model)]
pub struct UserViewModel {
    pub user: User,
    pub is_saving: bool,
    pub error: Option<SharedString>,
    repo: Arc<dyn UserRepo>,
}

impl UserViewModel {
    // ✅ 纯逻辑：可独立单测
    pub fn can_save(&self) -> bool {
        !self.is_saving && !self.user.email.is_empty()
    }

    // ✅ 命令：依赖 trait，可注入 mock
    #[command]
    pub fn save(&mut self, cx: &mut ViewContext<Self>) {
        if !self.can_save() { return; }
        self.is_saving = true;
        self.error = None;
        cx.notify();
        let repo = self.repo.clone();
        let user = self.user.clone();
        cx.spawn(|this, mut cx| async move {
            let result = repo.save(&user).await;
            let _ = this.update(&mut cx, |this, cx| {
                this.is_saving = false;
                match result {
                    Ok(_) => this.user.saved = true,
                    Err(e) => this.error = Some(e.to_string().into()),
                }
                cx.notify();
            });
        }).detach();
    }
}
```

### 单元测试

```rust
#[cfg(test)]
mod tests {
    use super::*;

    struct MockUserRepo { should_fail: bool }
    impl UserRepo for MockUserRepo {
        fn save(&self, _user: &User) -> Task<Result<()>> {
            Task::ready(if self.should_fail {
                Err(anyhow!("mock error"))
            } else { Ok(()) })
        }
    }

    #[test]
    fn can_save_returns_false_when_saving() {
        let vm = UserViewModel {
            user: User { email: "a@b.com".into(), ..Default::default() },
            is_saving: true,
            error: None,
            repo: Arc::new(MockUserRepo { should_fail: false }),
        };
        assert!(!vm.can_save());
    }

    #[test]
    fn can_save_returns_false_when_email_empty() {
        let vm = UserViewModel {
            user: User { email: "".into(), ..Default::default() },
            is_saving: false,
            error: None,
            repo: Arc::new(MockUserRepo { should_fail: false }),
        };
        assert!(!vm.can_save());
    }
}
```

纯逻辑方法可在无 GPUI 环境下测试，速度快、覆盖率高。

## 9.5.3 Model 的纯函数测试

Model 是纯数据，测试最简单：

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn todo_is_overdue_when_past_due_date() {
        let now = Utc::now();
        let todo = TodoItem {
            due_date: Some(now - Duration::days(1)),
            ..Default::default()
        };
        assert!(todo.is_overdue(now));
    }

    #[test]
    fn todo_not_overdue_without_due_date() {
        let todo = TodoItem { due_date: None, ..Default::default() };
        assert!(!todo.is_overdue(Utc::now()));
    }
}
```

Model 测试不依赖任何框架，应当覆盖所有边界条件。

## 9.5.4 组件快照测试

组件的 `.rml` 渲染结果可被快照化，确保 UI 不被意外改坏。

```rust
#[cfg(test)]
mod tests {
    use rml_testing::snapshot_view;

    #[test]
    fn button_renders_label() {
        let view = Button::new("Save".into());
        snapshot_view(view, "button-save").assert_matches();
    }

    #[test]
    fn button_renders_disabled_state() {
        let mut view = Button::new("Save".into());
        view.disabled = true;
        snapshot_view(view, "button-disabled").assert_matches();
    }
}
```

首次运行生成快照，后续运行比对。UI 主动变更时用 `RML_UPDATE_SNAPSHOTS=1` 更新。

## 9.5.5 集成测试：视图 + Service

集成测试验证 ViewModel 与真实 Service（或其 mock）的协作：

```rust
// tests/login_flow.rs
use my_app::views::login::LoginViewModel;
use my_app::services::MockAuthRepo;

#[test]
fn login_flow_shows_error_on_failure() {
    let mut cx = TestContext::new();
    cx.set_global(MockAuthRepo { should_fail: true });

    let mut vm = LoginViewModel::new(cx.clone());
    vm.email = "a@b.com".into();
    vm.password = "wrong".into();
    vm.login(&SubmitEvent::default(), &mut cx.with_view(&mut vm));

    cx.run_until_parked(); // 等待异步任务完成

    assert!(vm.error.is_some());
    assert!(!vm.is_loading);
}
```

`TestContext` 是 RML 提供的测试工具，模拟 `ViewContext` 但不真正渲染。

## 9.5.6 测试覆盖策略

| 层          | 测试类型   | 覆盖目标                          |
| ---------- | ------ | ----------------------------- |
| Model      | 单元     | 所有派生方法、边界条件                   |
| ViewModel  | 单元     | 所有 `#[computed]`、命令的状态机分支     |
| Service    | 单元     | 用 mock HTTP / 文件系统测试 I/O 路径   |
| Component  | 快照     | 关键状态组合的渲染输出                   |
| View+Service | 集成   | 关键用户流程（登录、提交、加载）              |
| 全应用        | E2E    | 关键业务路径（端到端）                   |

## 9.5.7 可测试性的设计准则

1. **纯逻辑与副作用分离**：能纯函数的就纯函数
2. **依赖 trait 而非具体类型**：Service 必须有 trait
3. **状态机显式化**：用枚举而非 `bool` 组合表达状态
4. **避免隐式全局**：少用 `cx.global`，多用注入
5. **命令方法短小**：一个命令只做一件事，便于覆盖分支

## 9.5.8 何时不必测试

- **`.rml` 模板**：模板是声明，逻辑在 ViewModel，模板测试由快照覆盖
- **纯样式**：样式变更由视觉走查
- **一次性脚本**：无复用价值的代码

测试的目的是**支撑重构**，对不会变化的代码过度测试是浪费。

下一节 → [9.6 反模式与代码异味](./anti-patterns.md)
