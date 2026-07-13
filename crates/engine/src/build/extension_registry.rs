//! 扩展组件自动发现与注册。
//!
//! 在 `Builder::build()` 编译 RML 前，自动扫描依赖树中所有 crate 的 Cargo.toml，
//! 解析 `[rml.metadata]` 声明的扩展组件并注册到动态注册表。
//!
//! 扩展 crate 只需在自己的 Cargo.toml 中声明：
//!
//! ```toml
//! [rml.metadata]
//! components = [
//!     { tag = "Terminal", ctor_path = "rml_ui_term::TerminalView", kind = "EntityRef", container = false },
//! ]
//! ```
//!
//! 使用方在 Cargo.toml 添加依赖即可生效，build.rs 无需任何注册代码。

use serde::Deserialize;

use crate::tags::{ComponentKind, ComponentTag, register_extension_component};

/// `[rml.metadata]` 的反序列化结构。
#[derive(Deserialize)]
struct RmlManifest {
    rml: Option<RmlSection>,
}

#[derive(Deserialize)]
struct RmlSection {
    metadata: Option<RmlMetadata>,
}

#[derive(Deserialize)]
struct RmlMetadata {
    components: Vec<ComponentEntry>,
}

#[derive(Deserialize)]
struct ComponentEntry {
    tag: String,
    ctor_path: String,
    kind: String,
    #[serde(default)]
    container: bool,
    /// `kind = "Stateful"` 或 `"StatefulWithDelegate"` 时需要。
    #[serde(default)]
    state_field: Option<String>,
    /// `kind = "Stateful"` 或 `"StatefulWithDelegate"` 时需要。
    #[serde(default)]
    state_ctor: Option<String>,
    /// `kind = "StatefulWithDelegate"` 时需要。
    #[serde(default)]
    delegate_attr: Option<String>,
}

/// 将 `String` 转为 `&'static str`（build.rs 进程短生命周期，leak 可接受）。
fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

/// 解析 `ComponentEntry` 并注册到动态注册表。
fn register_entry(entry: ComponentEntry, manifest_path: &str) {
    let kind = match entry.kind.as_str() {
        "EntityRef" => ComponentKind::EntityRef,
        "Stateless" => ComponentKind::Stateless,
        "StatelessNoId" => ComponentKind::StatelessNoId,
        "StatelessWithItems" => ComponentKind::StatelessWithItems,
        "StatefulWithDelegate" => {
            let state_field = entry
                .state_field
                .unwrap_or_else(|| panic!(
                    "Component '{}' in {} has kind=StatefulWithDelegate but missing state_field",
                    entry.tag, manifest_path
                ));
            let state_ctor = entry
                .state_ctor
                .unwrap_or_else(|| panic!(
                    "Component '{}' in {} has kind=StatefulWithDelegate but missing state_ctor",
                    entry.tag, manifest_path
                ));
            let delegate_attr = entry
                .delegate_attr
                .unwrap_or_else(|| panic!(
                    "Component '{}' in {} has kind=StatefulWithDelegate but missing delegate_attr",
                    entry.tag, manifest_path
                ));
            ComponentKind::StatefulWithDelegate {
                state_field: leak_str(state_field),
                state_ctor: leak_str(state_ctor),
                delegate_attr: leak_str(delegate_attr),
            }
        }
        "Stateful" => {
            let state_field = entry
                .state_field
                .unwrap_or_else(|| panic!(
                    "Component '{}' in {} has kind=Stateful but missing state_field",
                    entry.tag, manifest_path
                ));
            let state_ctor = entry
                .state_ctor
                .unwrap_or_else(|| panic!(
                    "Component '{}' in {} has kind=Stateful but missing state_ctor",
                    entry.tag, manifest_path
                ));
            ComponentKind::Stateful {
                state_field: leak_str(state_field),
                state_ctor: leak_str(state_ctor),
            }
        }
        other => panic!(
            "Unknown component kind '{}' for tag '{}' in {}",
            other, entry.tag, manifest_path
        ),
    };

    let tag = leak_str(entry.tag);
    let component = ComponentTag {
        ctor_path: leak_str(entry.ctor_path),
        kind,
        container: entry.container,
    };
    register_extension_component(tag, component);
}

/// 自动扫描依赖树并注册扩展组件。
///
/// 调用 `cargo metadata` 获取所有依赖包的 manifest 路径，
/// 逐个解析 `[rml.metadata]` 声明的组件。
pub fn auto_register_extensions() {
    let output = match std::process::Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .output()
    {
        Ok(o) => o,
        Err(e) => {
            eprintln!("rml: failed to run `cargo metadata`: {}", e);
            return;
        }
    };

    if !output.status.success() {
        eprintln!(
            "rml: `cargo metadata` failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        return;
    }

    let metadata: serde_json::Value = match serde_json::from_slice(&output.stdout) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("rml: failed to parse `cargo metadata` output: {}", e);
            return;
        }
    };

    let packages = match metadata["packages"].as_array() {
        Some(arr) => arr,
        None => return,
    };

    for package in packages {
        let manifest_path = match package["manifest_path"].as_str() {
            Some(p) => p,
            None => continue,
        };

        let content = match std::fs::read_to_string(manifest_path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        let rml_manifest: RmlManifest = match toml::from_str(&content) {
            Ok(m) => m,
            Err(_) => continue,
        };

        let Some(rml) = rml_manifest.rml else {
            continue;
        };
        let Some(metadata) = rml.metadata else {
            continue;
        };

        // Cargo.toml 变更时重新执行 build.rs
        println!("cargo:rerun-if-changed={}", manifest_path);

        for entry in metadata.components {
            register_entry(entry, manifest_path);
        }
    }
}
