//! rust-analyzer AnalysisHost 生命周期管理
//!
//! 通过 `ra_ap_load_cargo::load_workspace_at` 加载 Cargo workspace，
//! 管理 `AnalysisHost` + `Vfs` 生命周期。
//! 所有 `ra_ap_*` 类型仅在本模块（与 `adapter.rs`）内出现，不外泄。

use std::path::PathBuf;
use std::sync::Mutex;

use anyhow::Result;
use ra_ap_ide::AnalysisHost;
use ra_ap_ide_db::RootDatabase;
use ra_ap_load_cargo::{load_workspace_at, LoadCargoConfig, ProcMacroServerChoice};
use ra_ap_project_model::CargoConfig;
use ra_ap_vfs::Vfs;

/// RA 后端句柄：持有 AnalysisHost 与 Vfs
///
/// 加载耗时长（30s+），加载完成后 `is_ready()` 返回 true。
/// 所有查询通过 `analysis()` 获取只读 `Analysis` 快照。
pub struct RaHost {
    inner: Mutex<RaHostInner>,
}

struct RaHostInner {
    /// 加载完成前为 None
    host: Option<AnalysisHost>,
    vfs: Option<Vfs>,
    ready: bool,
}

impl RaHost {
    /// 创建一个未加载的后端。需调用 `load()` 才能查询。
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(RaHostInner {
                host: None,
                vfs: None,
                ready: false,
            }),
        }
    }

    /// 加载指定路径的 Cargo workspace
    ///
    /// `workspace_path` 应为包含 `Cargo.toml` 的目录或文件。
    /// 加载耗时较长（首次 30s+），调用方应在后台线程执行。
    pub fn load(&self, workspace_path: PathBuf) -> Result<()> {
        let load_config = LoadCargoConfig {
            load_out_dirs_from_check: true,
            with_proc_macro_server: ProcMacroServerChoice::Sysroot,
            prefill_caches: false,
            num_worker_threads: 2,
            proc_macro_processes: 1,
        };
        let cargo_config = CargoConfig::default();

        let (db, vfs, _proc_macro) = load_workspace_at(
            &workspace_path,
            &cargo_config,
            &load_config,
            &|_msg: String| {},
        )?;

        let host = AnalysisHost::with_database(db);
        let mut inner = self.inner.lock().expect("RaHost mutex poisoned");
        inner.host = Some(host);
        inner.vfs = Some(vfs);
        inner.ready = true;
        Ok(())
    }

    /// 是否已就绪（workspace 加载完成）
    pub fn is_ready(&self) -> bool {
        self.inner
            .lock()
            .map(|i| i.ready)
            .unwrap_or(false)
    }

    /// 获取只读 Analysis 快照（执行查询用）
    pub fn analysis(&self) -> Option<ra_ap_ide::Analysis> {
        let inner = self.inner.lock().ok()?;
        inner.host.as_ref().map(|h| h.analysis())
    }

    /// 获取 Vfs 引用（用于 FileId 解析）
    pub fn with_vfs<R>(&self, f: impl FnOnce(&Vfs) -> R) -> Option<R> {
        let inner = self.inner.lock().ok()?;
        inner.vfs.as_ref().map(f)
    }

    /// 获取 RootDatabase 引用（用于 file_text / line_index 查询）
    pub fn with_db<R>(&self, f: impl FnOnce(&RootDatabase) -> R) -> Option<R> {
        let inner = self.inner.lock().ok()?;
        inner.host.as_ref().map(|h| f(h.raw_database()))
    }
}

impl Default for RaHost {
    fn default() -> Self {
        Self::new()
    }
}
