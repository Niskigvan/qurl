use crate::utils::events::{
    io::IoEvent,
    key::{
        Key,
        Mod::{self, Alt, Any, Clean, Ctrl, Shift},
        Mouse,
    },
    term::UiEvent,
};
use async_trait::async_trait;
use std::{cell::RefCell, sync::mpsc::Sender};

use async_std::{
    future::Future,
    io::{prelude::BufReadExt, Lines},
    sync::{Arc, Mutex, MutexGuard},
};
use std::cmp::{max, min};
use syntect::highlighting::{Style as SyntStyle, ThemeSet};
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
#[derive(PartialEq, Clone, Debug, Default)]
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

#[derive(Clone, Debug)]
pub struct App {
    pub inp_data: Data,
    pub out_data: Data,
    pub schema_data: Data,

    pub size: Rect,
    pub input_cursor_position: u16,

    pub jq_input: String,
    pub changed: bool,
    pub running: bool,
}
impl Default for App {
    fn default() -> Self {
        App {
            // input: None,
            // output: None,
            io_tx: None,

            inp_data: Data::default(),
            out_data: Data::default(),
            schema_data: Data {
                format: DataFmt::SCHEMA,
                ..Default::default()
            },

            size: Rect::default(),
            input_cursor_position: 0,

            jq_input: ".".to_string(),
            changed: true,
            running: true,
        }
    }
}
