//! eDNA 业务案例 — Model 层（纯数据与表格行构建，不含视图逻辑）

use rml_ui::TableRow;

pub mod status {
    pub const COMPLETED: &str = "completed";
    pub const RUNNING: &str = "running";
    pub const WAITING: &str = "waiting";
}

pub mod event {
    pub const LEVEL_WARNING: &str = "warning";
    pub const LEVEL_INFO: &str = "info";
    pub const STATUS_RECOVERED: &str = "recovered";
    pub const STATUS_NORMAL: &str = "normal";
}

/// 折线图数据点
#[derive(Clone, Debug)]
pub struct ChartPoint {
    pub label: String,
    pub value: f64,
}

/// 实时指标序列（含历史，供 LineChart 使用）
#[derive(Clone, Debug)]
pub struct MetricSeries {
    pub title: String,
    pub unit: String,
    pub range: String,
    pub current: f64,
    pub history: Vec<ChartPoint>,
}

impl MetricSeries {
    pub fn new(title: &str, unit: &str, range: &str, initial: f64) -> Self {
        let history = (0..8)
            .map(|i| ChartPoint {
                label: format!("t-{i}"),
                value: initial + (i as f64 - 4.0) * 0.05,
            })
            .collect();
        Self {
            title: title.into(),
            unit: unit.into(),
            range: range.into(),
            current: initial,
            history,
        }
    }

    pub fn push_sample(&mut self, value: f64) {
        self.current = value;
        self.history.push(ChartPoint {
            label: format!("{}", self.history.len()),
            value,
        });
        if self.history.len() > 12 {
            self.history.remove(0);
        }
    }

    pub fn display_value(&self) -> String {
        match self.unit.as_str() {
            "°C" | "m" | "L" => format!("{:.1} {}", self.current, self.unit),
            "mg/L" | "kPa" => format!("{:.2} {}", self.current, self.unit),
            "L/min" => format!("{:.2} {}", self.current, self.unit),
            _ => format!("{:.1} {}", self.current, self.unit),
        }
    }
}

/// 初始 6 项实时指标
pub fn initial_metric_series() -> Vec<MetricSeries> {
    vec![
        MetricSeries::new("温度", "°C", "0 - 40 °C", 18.6),
        MetricSeries::new("水位", "m", "0 - 5 m", 2.35),
        MetricSeries::new("溶解氧", "mg/L", "0 - 15 mg/L", 8.42),
        MetricSeries::new("压力", "kPa", "0 - 100 kPa", 42.7),
        MetricSeries::new("流量", "L/min", "0 - 5 L/min", 1.86),
        MetricSeries::new("累积流量", "L", "0 - 500 L", 128.4),
    ]
}

/// 构建通道计划表行
pub fn build_channel_rows(current_channel: u32, running_progress: u32) -> Vec<TableRow> {
    (1..=16)
        .map(|id| build_channel_row(id, current_channel, running_progress))
        .collect()
}

fn build_channel_row(id: u32, current_channel: u32, running_progress: u32) -> TableRow {
    let (status, status_kind, progress_text, progress_val) = if id < current_channel {
        (
            "已完成".to_string(),
            status::COMPLETED,
            "100%".to_string(),
            "100".to_string(),
        )
    } else if id == current_channel {
        (
            "运行中".to_string(),
            status::RUNNING,
            format!("{running_progress}%"),
            running_progress.to_string(),
        )
    } else {
        (
            "等待中".to_string(),
            status::WAITING,
            "0%".to_string(),
            "0".to_string(),
        )
    };
    TableRow::new()
        .cell("id", id.to_string())
        .cell("enabled", "是")
        .cell("time", format!("10:{:02}", 30 + id))
        .cell("order", id.to_string())
        .cell("status", status)
        .cell("status_kind", status_kind)
        .cell("volume", "6.00 L")
        .cell("progress", progress_text)
        .cell("progress_val", progress_val)
}

/// 由指标序列构建表格行
pub fn build_metric_rows(series: &[MetricSeries]) -> Vec<TableRow> {
    series
        .iter()
        .map(|s| {
            TableRow::new()
                .cell("title", s.title.clone())
                .cell("value", s.display_value())
                .cell("range", s.range.clone())
        })
        .collect()
}

/// 构建事件日志表行
pub fn build_event_rows() -> Vec<TableRow> {
    vec![
        TableRow::new()
            .cell("time", "2025-05-27 10:22:15")
            .cell("level", "警告")
            .cell("level_kind", event::LEVEL_WARNING)
            .cell("message", "水位低于阈值 2.20 m")
            .cell("status", "已恢复")
            .cell("status_kind", event::STATUS_RECOVERED),
        TableRow::new()
            .cell("time", "2025-05-27 10:20:03")
            .cell("level", "提示")
            .cell("level_kind", event::LEVEL_INFO)
            .cell("message", "压力正常 42.6 kPa")
            .cell("status", "正常")
            .cell("status_kind", event::STATUS_NORMAL),
        TableRow::new()
            .cell("time", "2025-05-27 10:15:47")
            .cell("level", "提示")
            .cell("level_kind", event::LEVEL_INFO)
            .cell("message", "系统启动完成")
            .cell("status", "正常")
            .cell("status_kind", event::STATUS_NORMAL),
    ]
}

/// 工艺流程示意图资源路径（嵌入 assets）
pub const PROCESS_DIAGRAM_SRC: &str = "edna/process-flow.svg";
