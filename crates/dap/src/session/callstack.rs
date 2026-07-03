//! 调用栈状态
//!
//! 缓存引擎上报的线程列表与选中线程的栈帧。状态由 `DebugSession` 在收到
//! `stopped` 事件后调用 `refresh_from_engine` 更新。

use std::collections::HashMap;

use crate::engine::{StackFrame, Thread};

/// 调用栈缓存
#[derive(Default)]
pub struct CallStack {
    threads: Vec<Thread>,
    /// 每个线程的栈帧缓存（按 thread_id 索引）
    frames: HashMap<u64, Vec<StackFrame>>,
}

impl CallStack {
    pub fn new() -> Self {
        Self::default()
    }

    /// 更新线程列表
    pub fn set_threads(&mut self, threads: Vec<Thread>) {
        let live_ids: Vec<u64> = threads.iter().map(|t| t.id).collect();
        self.threads = threads;
        self.frames.retain(|id, _| live_ids.contains(id));
    }

    /// 更新指定线程的栈帧
    pub fn set_frames(&mut self, thread_id: u64, frames: Vec<StackFrame>) {
        self.frames.insert(thread_id, frames);
    }

    /// 获取线程列表（只读）
    pub fn threads(&self) -> &[Thread] {
        &self.threads
    }

    /// 获取指定线程的栈帧（只读）
    pub fn frames(&self, thread_id: u64) -> &[StackFrame] {
        self.frames
            .get(&thread_id)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// 清空所有缓存
    pub fn clear(&mut self) {
        self.threads.clear();
        self.frames.clear();
    }
}
