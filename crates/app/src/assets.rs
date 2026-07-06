//! 复合 `AssetSource` —— 桥接 gpui-component-assets 与 RML 嵌入资源
//!
//! GPUI 的 `svg().path("...")` 通过 `Application::with_assets` 注册的 `AssetSource`
//! 解析路径。gpui-component-assets 仅包含 `icons/**/*.svg`（内置图标），
//! 用户嵌入的 `assets/logo.svg` 等资源由 `rml_core::assets::load` 管理，
//! 不在 GPUI AssetSource 的可见范围内。
//!
//! 本模块提供 [`CompositeAssets`]，按以下顺序解析路径：
//! 1. `gpui_component_assets::Assets`（内置图标，如 `icons/foo.svg`）
//! 2. `rml_core::assets::load`（用户嵌入资源，如 `logo.svg`、`themes/dark.css`）
//!
//! `RmlApplication::run` 自动使用 `CompositeAssets`，无需用户手动配置。

use std::borrow::Cow;

use gpui::{AssetSource, Result, SharedString};

/// 复合资源源：gpui-component 内置图标 + RML 用户嵌入资源
///
/// 由 `RmlApplication::run` 自动注册，使 `Icon::empty().path("logo.svg")`
/// 等用户资源路径能被 GPUI 的 svg 渲染器解析。
pub struct CompositeAssets;

impl AssetSource for CompositeAssets {
    fn load(&self, path: &str) -> Result<Option<Cow<'static, [u8]>>> {
        if path.is_empty() {
            return Ok(None);
        }
        // 1. 内置图标（icons/**/*.svg）
        //    gpui_component_assets::Assets.load 对未找到的路径返回 Err（而非 Ok(None)），
        //    用 .ok().flatten() 将 Err 转为 None，继续尝试 rml_core::assets::load。
        if let Some(data) = gpui_component_assets::Assets.load(path).ok().flatten() {
            return Ok(Some(data));
        }
        // 2. RML 用户嵌入资源
        if let Some(data) = rml_core::assets::load(path) {
            return Ok(Some(Cow::Borrowed(data)));
        }
        Ok(None)
    }

    fn list(&self, path: &str) -> Result<Vec<SharedString>> {
        let mut entries = gpui_component_assets::Assets.list(path)?;
        // 追加 RML 嵌入资源中此前未列出的路径（前缀匹配）
        for p in rml_core::assets::list() {
            if p.starts_with(path) && !entries.iter().any(|e| e.as_ref() == p) {
                entries.push(p.into());
            }
        }
        Ok(entries)
    }
}
