//! DebugClient —— 调试服务客户端（DAP 协议）骨架
//!
//! 与 `LanguageClient` 同构：封装 debug adapter 进程 + DAP IPC + 调试能力工厂。
//! 当前为骨架，后续实现 DAP 协议后补齐方法。
//!
//! ## 设计预览（后续实现）
//!
//! ```ignore
//! use rust_rml_client::DebugClient;
//!
//! let client = DebugClient::unified(&workspace_root)?;
//! client.launch(&program_path, &args);
//! client.set_breakpoints(&file_path, &line_numbers);
//! client.continue_execution();
//! ```

use std::path::Path;

use anyhow::Result;

use crate::language_profile::DebugProfile;

/// DebugClient —— 调试服务客户端（DAP 协议）
///
/// 与 `LanguageClient` 同构：封装 debug adapter 进程 + DAP IPC + 调试能力工厂。
/// 当前为骨架，后续实现 DAP 协议后补齐方法。
pub struct DebugClient {
    profile: DebugProfile,
    // dap: Arc<DapClient>,  // 后续引入 DAP client
}

impl DebugClient {
    /// 通用构造：按 profile 启动 DAP adapter 并完成 attach 握手
    pub fn new(profile: DebugProfile, _workspace_root: &Path) -> Result<Self> {
        // 后续实现：
        // 1. spawn DAP adapter 子进程（profile 驱动二进制解析）
        // 2. DAP initialize 握手
        // 3. attach / launch 配置
        let _ = &profile;
        todo!("DAP implementation in future phase")
    }

    /// rust+rml 一体化便捷构造 —— `DebugProfile::unified()` 预设（lldb-vscode）
    pub fn unified(workspace_root: &Path) -> Result<Self> {
        Self::new(DebugProfile::unified(), workspace_root)
    }

    /// 调试 profile
    pub fn profile(&self) -> &DebugProfile {
        &self.profile
    }
}
