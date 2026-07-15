use std::sync::Arc;
use std::time::Duration;

use gpui::{AnyElement, App, Hsla, IntoElement, ParentElement, SharedString, Styled, Window, div, px};
use gpui_component::ActiveTheme as _;
use gpui_component::StyledExt as _;
use gpui_component::chart::LineChart;
use rml::prelude::*;
use rml_core::i18n::t_static;
use rml_ui::{Progress, Sizable, TableColumn, TableDelegate, TableRow, Tag, h_flex, v_flex};

use super::model::{self, event, status, ChartPoint, MetricSeries, PROCESS_DIAGRAM_SRC};

// ── TableDelegate：通道表状态/进度列 ─────────────────────────────────────

/// 通道表单元格委托 —— 状态列 Tag、运行中进度列 Progress（其余列默认文本）
#[derive(Default, Clone)]
pub struct EdnaChannelTableDelegate;

impl TableDelegate for EdnaChannelTableDelegate {
    fn render_cell(
        &self,
        _row: usize,
        _col: usize,
        column: &TableColumn,
        row_data: &TableRow,
        _cx: &mut App,
    ) -> AnyElement {
        let key = column.key.as_ref();
        match key {
            "status" => render_status_tag(row_data),
            "progress" => render_progress_cell(row_data),
            _ => default_text_cell(row_data, &column.key),
        }
    }
}

fn render_status_tag(row_data: &TableRow) -> AnyElement {
    let kind = row_data.get(&SharedString::from("status_kind"));
    let text = row_data.get(&SharedString::from("status"));
    let tag = match kind.as_ref() {
        status::COMPLETED => Tag::success(),
        status::RUNNING => Tag::info(),
        _ => Tag::secondary(),
    };
    tag.small().child(text).into_any_element()
}

fn render_progress_cell(row_data: &TableRow) -> AnyElement {
    let kind = row_data.get(&SharedString::from("status_kind"));
    if kind.as_ref() == status::RUNNING {
        let val: f32 = row_data
            .get(&SharedString::from("progress_val"))
            .parse()
            .unwrap_or(35.0);
        return Progress::new("edna-ch-progress")
            .value(val)
            .xsmall()
            .into_any_element();
    }
    default_text_cell(row_data, &SharedString::from("progress"))
}

// ── TableDelegate：事件日志类型/状态列 ───────────────────────────────────

#[derive(Default, Clone)]
pub struct EdnaEventTableDelegate;

impl TableDelegate for EdnaEventTableDelegate {
    fn render_cell(
        &self,
        _row: usize,
        _col: usize,
        column: &TableColumn,
        row_data: &TableRow,
        _cx: &mut App,
    ) -> AnyElement {
        match column.key.as_ref() {
            "level" => render_event_level_tag(row_data),
            "status" => render_event_status_tag(row_data),
            _ => default_text_cell(row_data, &column.key),
        }
    }
}

fn render_event_level_tag(row_data: &TableRow) -> AnyElement {
    let kind = row_data.get(&SharedString::from("level_kind"));
    let text = row_data.get(&SharedString::from("level"));
    let tag = match kind.as_ref() {
        event::LEVEL_WARNING => Tag::warning(),
        _ => Tag::success(),
    };
    tag.small().child(text).into_any_element()
}

fn render_event_status_tag(row_data: &TableRow) -> AnyElement {
    let kind = row_data.get(&SharedString::from("status_kind"));
    let text = row_data.get(&SharedString::from("status"));
    let tag = match kind.as_ref() {
        event::STATUS_RECOVERED => Tag::info().outline(),
        _ => Tag::success().outline(),
    };
    tag.small().child(text).into_any_element()
}

fn default_text_cell(row_data: &TableRow, key: &SharedString) -> AnyElement {
    div()
        .overflow_hidden()
        .child(row_data.get(key))
        .into_any_element()
}

// ── ViewModel ────────────────────────────────────────────────────────────

#[contribute(
    host_id = "demo.shell",
    id = "business.edna",
    kind = "case",
    group = "business",
    order = 10,
)]
#[component]
pub struct EdnaCase {
    // 设备概览
    pub device_status: String,
    pub current_time: String,
    pub comm_status: String,

    // 运行状态
    pub current_channel: u32,
    pub total_channels: u32,
    pub current_stage: String,
    pub running_time: String,
    pub stage_countdown: String,
    pub stage_countdown_seconds: u32,
    pub alarm_status: String,
    pub filter_status: String,
    pub start_threshold: String,
    pub last_filter_value: String,
    pub round_progress: f32,
    pub round_channels_done: u32,
    pub round_channels_total: u32,
    pub refresh_interval: String,
    pub process_step_index: usize,

    // 实时指标与刷新
    pub metric_series: Vec<MetricSeries>,
    pub chart_history: Vec<ChartPoint>,
    pub sampling_active: bool,
    pub channel_running_progress: u32,
    pub running_seconds: u32,
    pub tick_counter: u32,

    // 参数表单（双向绑定）
    pub target_volume: String,
    pub cleaning_time: String,
    pub filter_volume: String,
    pub purge_time: String,
    pub preservative_volume: String,
    pub power_off_order: bool,
    pub selected_channel_order: String,
    pub channel_order_items: rml_ui::SearchableVec<SharedString>,

    // 表格数据（View 只读绑定）
    pub channel_columns: Vec<TableColumn>,
    pub channel_rows: Vec<TableRow>,
    pub channel_delegate: Arc<dyn TableDelegate>,
    pub metric_columns: Vec<TableColumn>,
    pub metric_rows: Vec<TableRow>,
    pub event_columns: Vec<TableColumn>,
    pub event_rows: Vec<TableRow>,
    pub event_delegate: Arc<dyn TableDelegate>,
}

impl Default for EdnaCase {
    fn default() -> Self {
        Self {
            device_status: String::new(),
            current_time: String::new(),
            comm_status: String::new(),
            current_channel: 0,
            total_channels: 16,
            current_stage: String::new(),
            running_time: String::new(),
            stage_countdown: String::new(),
            stage_countdown_seconds: 0,
            alarm_status: String::new(),
            filter_status: String::new(),
            start_threshold: String::new(),
            last_filter_value: String::new(),
            round_progress: 0.0,
            round_channels_done: 0,
            round_channels_total: 8,
            refresh_interval: String::new(),
            process_step_index: 0,
            metric_series: Vec::new(),
            chart_history: Vec::new(),
            sampling_active: true,
            channel_running_progress: 35,
            running_seconds: 516,
            tick_counter: 0,
            target_volume: String::new(),
            cleaning_time: String::new(),
            filter_volume: String::new(),
            purge_time: String::new(),
            preservative_volume: String::new(),
            power_off_order: false,
            selected_channel_order: String::new(),
            channel_order_items: rml_ui::SearchableVec::new(Vec::<SharedString>::new()),
            channel_columns: Vec::new(),
            channel_rows: Vec::new(),
            channel_delegate: Arc::new(EdnaChannelTableDelegate),
            metric_columns: Vec::new(),
            metric_rows: Vec::new(),
            event_columns: Vec::new(),
            event_rows: Vec::new(),
            event_delegate: Arc::new(EdnaEventTableDelegate),
            __rml_state: Default::default(),
        }
    }
}

impl IContribution for EdnaCase {
    fn id(&self) -> &str {
        Self::CONTRIBUTION_ID
    }
    fn name(&self) -> SharedString {
        t_static("case.edna.title")
    }
}

impl ILifecycle for EdnaCase {
    fn on_loaded(&mut self, _window: &mut gpui::Window, cx: &mut Context<Self>) {
        self.init_device_state();
        self.init_tables();
        self.init_form_defaults();
        self.start_refresh_timer(cx);
    }
}

impl EdnaCase {
    fn init_device_state(&mut self) {
        self.device_status = "运行中".into();
        self.current_time = "2025-05-27 10:24:36".into();
        self.comm_status = "正常".into();
        self.current_channel = 3;
        self.total_channels = 16;
        self.current_stage = "正式过滤".into();
        self.running_time = format_duration(self.running_seconds);
        self.stage_countdown_seconds = 21 * 60 + 24;
        self.stage_countdown = format_duration(self.stage_countdown_seconds);
        self.alarm_status = "无报警".into();
        self.filter_status = "智能滤芯健康".into();
        self.start_threshold = "≥ 6.00 mg/L".into();
        self.last_filter_value = "8.45 mg/L".into();
        self.round_progress = 37.0;
        self.round_channels_done = 3;
        self.round_channels_total = 8;
        self.refresh_interval = "1s".into();
        self.process_step_index = 2;
    }

    fn init_form_defaults(&mut self) {
        self.target_volume = "6.00".into();
        self.cleaning_time = "60".into();
        self.filter_volume = "10.00".into();
        self.purge_time = "2.00".into();
        self.preservative_volume = "25".into();
        self.power_off_order = true;
        self.selected_channel_order = "3".into();
        self.channel_order_items = rml_ui::SearchableVec::new(
            (1..=8)
                .map(|n| SharedString::from(n.to_string()))
                .collect::<Vec<_>>(),
        );
    }

    fn init_tables(&mut self) {
        self.channel_columns = vec![
            TableColumn::new("id", "通道").width(px(48.)),
            TableColumn::new("enabled", "启用").width(px(48.)),
            TableColumn::new("time", "采样时间").width(px(88.)),
            TableColumn::new("order", "顺序").width(px(48.)),
            TableColumn::new("status", "状态").width(px(80.)),
            TableColumn::new("volume", "目标体积").width(px(72.)),
            TableColumn::new("progress", "进度"),
        ];
        self.channel_rows = model::build_channel_rows(self.current_channel, self.channel_running_progress);

        self.metric_series = model::initial_metric_series();
        self.metric_columns = vec![
            TableColumn::new("title", "指标").width(px(80.)),
            TableColumn::new("value", "数值").width(px(100.)),
            TableColumn::new("range", "量程"),
        ];
        self.metric_rows = model::build_metric_rows(&self.metric_series);

        self.event_columns = vec![
            TableColumn::new("time", "时间").width(px(148.)),
            TableColumn::new("level", "类型").width(px(72.)),
            TableColumn::new("message", "描述"),
            TableColumn::new("status", "状态").width(px(80.)),
        ];
        self.event_rows = model::build_event_rows();
    }

    #[computed]
    pub fn channel_label(&self) -> String {
        format!("{} / {}", self.current_channel, self.total_channels)
    }

    #[computed]
    pub fn round_channels_label(&self) -> String {
        format!(
            "{} / {} 通道",
            self.round_channels_done, self.round_channels_total
        )
    }

    #[computed]
    pub fn round_progress_text(&self) -> String {
        format!("{:.0}%", self.round_progress)
    }

    #[computed]
    pub fn header_extra(&self) -> String {
        format!(
            "当前通道: {} / {}",
            self.current_channel, self.total_channels
        )
    }

    #[computed]
    pub fn refresh_label(&self) -> String {
        format!("数据刷新: {}", self.refresh_interval)
    }

    #[computed]
    pub fn process_diagram_src(&self) -> String {
        PROCESS_DIAGRAM_SRC.to_string()
    }

    fn start_refresh_timer(&self, cx: &mut Context<Self>) {
        cx.spawn(|this: gpui::WeakEntity<Self>, cx: &mut gpui::AsyncApp| {
            let mut cx = cx.clone();
            async move {
                loop {
                    cx.background_executor().timer(Duration::from_secs(1)).await;
                    let _ = this.update(&mut cx, |this, cx| {
                        this.on_refresh_tick(cx);
                    });
                }
            }
        })
        .detach();
    }

    fn on_refresh_tick(&mut self, cx: &mut Context<Self>) {
        self.tick_counter += 1;
        self.running_seconds += 1;
        self.running_time = format_duration(self.running_seconds);

        let base_secs = 10 * 3600 + 24 * 60 + 36 + self.tick_counter;
        let h = base_secs / 3600;
        let m = (base_secs % 3600) / 60;
        let s = base_secs % 60;
        self.current_time = format!("2025-05-27 {h:02}:{m:02}:{s:02}");

        if self.sampling_active {
            if self.stage_countdown_seconds > 0 {
                self.stage_countdown_seconds -= 1;
                self.stage_countdown = format_duration(self.stage_countdown_seconds);
            }
            if self.channel_running_progress < 100 {
                self.channel_running_progress += 1;
            } else if self.current_channel < self.total_channels {
                self.current_channel += 1;
                self.channel_running_progress = 1;
                if self.round_channels_done < self.round_channels_total {
                    self.round_channels_done += 1;
                }
            }
            self.round_progress =
                ((self.round_channels_done.saturating_sub(1) as f32 * 100.0)
                    + self.channel_running_progress as f32)
                    / self.round_channels_total as f32;
        }

        let bases = [18.6, 2.35, 8.42, 42.7, 1.86, 128.4];
        for (i, series) in self.metric_series.iter_mut().enumerate() {
            let phase = (self.tick_counter as f64 + i as f64 * 1.7) * 0.12;
            let value = bases[i] + phase.sin() * (0.05 + i as f64 * 0.01);
            series.push_sample(value);
        }
        self.chart_history = self
            .metric_series
            .first()
            .map(|s| s.history.clone())
            .unwrap_or_default();
        self.metric_rows = model::build_metric_rows(&self.metric_series);
        self.channel_rows = model::build_channel_rows(self.current_channel, self.channel_running_progress);
        cx.notify();
    }

    /// 6 项迷你 LineChart 面板（gpui-component Chart）
    ///
    /// RML 限制：`each` 循环内 `stroke={expr}` 仅接受 `Hsla` 值，无法按索引解析
    /// 主题色名（`chart_1`..`chart_5`）。待框架支持动态主题色名解析后回归声明式。
    pub fn render_metrics_panel(
        &self,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = metric_chart_colors(cx);
        let cards: Vec<AnyElement> = self
            .metric_series
            .iter()
            .enumerate()
            .map(|(i, series)| render_metric_card(series, colors[i % colors.len()], cx))
            .collect();

        div()
            .grid()
            .grid_cols(2u16)
            .gap(px(8.))
            .children(cards)
            .into_any_element()
    }

    fn do_start_sampling(&mut self, cx: &mut Context<Self>) {
        self.sampling_active = true;
        self.device_status = "运行中".into();
        self.process_step_index = 2;
        self.current_stage = "正式过滤".into();
        cx.notify();
    }

    fn do_stop(&mut self, cx: &mut Context<Self>) {
        self.sampling_active = false;
        self.device_status = "已停止".into();
        cx.notify();
    }

    fn do_manual_clean(&mut self, cx: &mut Context<Self>) {
        self.process_step_index = 1;
        self.current_stage = "清洗".into();
        cx.notify();
    }

    fn do_manual_purge(&mut self, cx: &mut Context<Self>) {
        self.process_step_index = 3;
        self.current_stage = "吹扫".into();
        cx.notify();
    }

    fn do_save_params(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn do_import_plan(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn do_export_plan(&mut self, cx: &mut Context<Self>) {
        cx.notify();
    }

    fn do_clear_log(&mut self, cx: &mut Context<Self>) {
        self.event_rows.clear();
        cx.notify();
    }

    #[command]
    pub fn on_start_sampling(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_start_sampling(cx);
    }

    #[command]
    pub fn on_stop(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_stop(cx);
    }

    #[command]
    pub fn on_manual_clean(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_manual_clean(cx);
    }

    #[command]
    pub fn on_manual_purge(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_manual_purge(cx);
    }

    #[command]
    pub fn on_save_params(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_save_params(cx);
    }

    #[command]
    pub fn on_import_plan(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_import_plan(cx);
    }

    #[command]
    pub fn on_export_plan(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_export_plan(cx);
    }

    #[command]
    pub fn on_clear_log(&mut self, _: &ClickEvent, cx: &mut Context<Self>) {
        self.do_clear_log(cx);
    }

    #[command]
    pub fn on_view_details(&mut self, _: &ClickEvent, _: &mut Context<Self>) {}

    #[command]
    pub fn on_toggle_power_off_order(&mut self, checked: &bool, _: &mut Context<Self>) {
        self.power_off_order = *checked;
    }
}

fn format_duration(secs: u32) -> String {
    format!(
        "{:02}:{:02}:{:02}",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

fn metric_chart_colors(cx: &App) -> Vec<Hsla> {
    vec![
        cx.theme().chart_1,
        cx.theme().chart_2,
        cx.theme().chart_3,
        cx.theme().chart_4,
        cx.theme().chart_5,
        cx.theme().info,
    ]
}

fn render_metric_card(series: &MetricSeries, stroke: Hsla, cx: &App) -> AnyElement {
    let data: Vec<ChartPoint> = series.history.clone();
    v_flex()
        .gap(px(4.))
        .p(px(8.))
        .border_1()
        .border_color(cx.theme().border)
        .rounded(cx.theme().radius)
        .child(
            h_flex()
                .justify_between()
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(series.title.clone()),
                )
                .child(
                    div()
                        .text_sm()
                        .font_semibold()
                        .child(series.display_value()),
                ),
        )
        .child(
            div()
                .h(px(52.))
                .w_full()
                .child(
                    LineChart::new(data)
                        .x(|p: &ChartPoint| p.label.clone())
                        .y(|p: &ChartPoint| p.value)
                        .stroke(stroke)
                        .tick_margin(2),
                ),
        )
        .child(
            div()
                .text_xs()
                .text_color(cx.theme().muted_foreground)
                .child(series.range.clone()),
        )
        .into_any_element()
}
