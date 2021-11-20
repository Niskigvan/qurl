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
    // input: Option<Arc<Read>>,
    // output: Option<Arc<Write>>,
    io_tx: Option<Sender<IoEvent>>,

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

impl App {
    pub fn new(io_tx: Sender<IoEvent>) -> Self {
        App {
            io_tx: Some(io_tx),
            ..Default::default()
        }
    }
    pub fn dispatch(&self, event: IoEvent) {
        // self.is_loading = true;
        // if let Some(io_tx) = &self.io_tx {
        //     if let Err(e) = io_tx.send(action) {
        //         self.is_loading = false;
        //         println!("Error from dispatch {}", e);
        //         // TODO: handle error
        //     };
        // }
    }
    pub fn on_resize(&mut self, size: Rect) {
        self.size = size;

        // Based on the size of the terminal, adjust the search limit.
        let potential_limit = max((self.size.height as i32) - 13, 0) as u32;
        let max_limit = min(potential_limit, 50);
        let large_search_limit = min((f32::from(size.height) / 1.4) as u32, max_limit);
        let small_search_limit = min((f32::from(size.height) / 2.85) as u32, max_limit / 2);

        /* app.dispatch(IoEvent::UpdateSearchLimits(
            large_search_limit,
            small_search_limit,
        )); */
    }
    pub fn on_input(&mut self, _events: &Vec<Mod>) {
        let mut cont = &mut self.inp_data;
        let max_row = cont.formatted_lines.len() as i32;
        for evt in _events {
            match evt {
                Ctrl(Key::Char('q')) => self.running = false,
                Clean(Key::MouseScroll(pos, x, y)) => {
                    cont.scroll.0 = (cont.scroll.0 as i32 + pos).max(0).min(max_row) as u16;
                }
                Shift(Key::MouseScroll(pos, x, y)) => {
                    cont.scroll.1 = (cont.scroll.1 as i32 + pos).max(0) as u16;
                }
                Clean(Key::MouseDown(Mouse::Middle, ..)) => {
                    cont.scroll.0 = 0u16;
                }
                Shift(Key::MouseDown(Mouse::Middle, ..)) => {
                    cont.scroll.1 = 0u16;
                }
                Clean(Key::Down) => {
                    cont.scroll.0 = (cont.scroll.0 as i32 + 1).min(max_row) as u16;
                }
                Clean(Key::Up) => {
                    cont.scroll.0 = (cont.scroll.0 as i32 - 1).max(0) as u16;
                }
                Shift(Key::Down) => {
                    cont.scroll.0 = (cont.scroll.0 as i32 + 10).min(max_row) as u16;
                }
                Shift(Key::Up) => {
                    cont.scroll.0 = (cont.scroll.0 as i32 - 10).max(0) as u16;
                }
                Any(Key::Char('g')) => {
                    cont.scroll.0 = 0;
                }
                Any(Key::Char('G')) => {
                    cont.scroll.0 = (max_row - self.size.height as i32) as u16 + 2;
                }
                _ => {}
            }
        }
    }
    pub fn on_tick(&mut self) {
        // if (self.input_cont.loaded && !self.pause) {
        //     self.pause = true;
        // }
    }
}

/* fn mutate<'a, F>(app: &'a Arc<Mutex<App>>, cb: F) -> impl std::future::Future<Output = ()>
where
    F: FnOnce(&'a mut MutexGuard<App + 'static>),
{
    async move {
        let mut st = app.lock().await;
        cb(&mut st);
        st.changed = true;
    }
}
 */
