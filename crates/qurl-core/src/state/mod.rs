use std::time::{Duration, Instant};

use syntect::highlighting::{Style as SyntStyle, ThemeSet};
use tui::backend::Backend;
use tui::layout::Alignment;
use tui::style::{Color, Style};
use tui::terminal::Frame;
use tui::widgets::{Block, Borders, Paragraph};
use tui::{layout::Rect, text::Spans};

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum DataFmt {
    JSON,
    YAML,
    XML,
    CSV,
    SCHEMA,
}
impl Default for DataFmt {
    fn default() -> Self {
        DataFmt::JSON
    }
}
#[derive(PartialEq, Debug, Default)]
pub struct Data {
    pub format: DataFmt,
    pub original_lines: Vec<String>,
    pub formatted_lines: Vec<Vec<(SyntStyle, String)>>,
    pub values: Vec<serde_json::Value>,
    pub scroll: (u16, u16),
    pub original_loaded: bool,
    pub values_loaded: bool,
    pub formatted_loaded: bool,
}

#[derive(PartialEq, Debug, Default)]
pub struct Options {
    pub tick_rate: Duration,
    pub frame_rate: Duration,
}

#[derive(PartialEq, Debug)]
pub struct App {
    pub options: Options,
    pub inp_data: Data,
    pub out_data: Data,
    pub schema_data: Data,

    pub size: Rect,
    pub input_cursor_position: u16,

    pub jq_input: String,
    pub mouse_pos: (u16, u16),
    pub last_render_at: Instant,
    pub need_render: bool,
    pub running: bool,
}
impl Default for App {
    fn default() -> Self {
        App {
            options: Options {
                tick_rate: Duration::from_millis(160),
                frame_rate: Duration::from_millis(33),
            },
            inp_data: Data::default(),
            out_data: Data::default(),
            schema_data: Data {
                format: DataFmt::SCHEMA,
                ..Default::default()
            },

            size: Rect::default(),
            input_cursor_position: 0,

            jq_input: ".".to_string(),
            mouse_pos: (0, 0),
            last_render_at: Instant::now(),
            need_render: false,
            running: true,
        }
    }
}
impl App {
    pub fn render<B: Backend>(&self, frame: &mut Frame<'_, B>) {
        let size = frame.size();
        // This is where you add new widgets.
        // See the following resources:
        // - https://docs.rs/tui/0.16.0/tui/widgets/index.html
        // - https://github.com/fdehau/tui-rs/tree/v0.16.0/examples
        frame.render_widget(
            Paragraph::new(format!("{}ms", self.last_render_at.elapsed().as_millis()))
                .block(Block::default().borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Center),
            frame.size(),
        );
        frame.render_widget(
            Block::default()
                .borders(Borders::NONE)
                .style(Style::default().fg(Color::White).bg(Color::LightYellow)),
            Rect {
                x: self.mouse_pos.0.min(size.width - 1),
                y: self.mouse_pos.1.min(size.height - 1),
                width: 1u16,
                height: 1u16,
            },
        );
    }
}
