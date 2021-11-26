mod events;
use crate::{
    actions::{
        AppAction,
        Interaction::{Key, Mod, Mouse},
    },
    state::App,
    AppStore,
};
use anyhow::{anyhow, Result};
use async_lock::Barrier;
use async_std::{
    future,
    prelude::*,
    sync::{Arc, Mutex, MutexGuard, RwLock},
    task::{self, Context, Poll},
};
use async_store::{Cond, Store};
use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
use events::InteractionStream;
use std::{
    io::{self, Stderr, Stdout},
    time::Instant,
};
use tui::backend::{Backend, CrosstermBackend};
use tui::Terminal;
/// Representation of a terminal user interface.
///
/// It is responsible for setting up the terminal,
/// initializing the interface and handling the draw events.

pub struct Tui {
    /// Interface to the Terminal.
    terminal: Terminal<CrosstermBackend<Stdout>>,
}

impl Tui {
    /// Constructs a new instance of [`Tui`].
    pub async fn run(store: AppStore) -> Result<()> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        let tui = Arc::new(Mutex::new(Tui { terminal }));
        let tui2 = tui.clone();
        tui.lock().await.init()?;
        let barrier = Arc::new(Barrier::new(2));

        let barrier2 = barrier.clone();
        let actions_task = task::spawn(InteractionStream::run(store.clone()));

        store
            .lock()
            .await
            .on_action(move |state, action| {
                task::spawn(async move {
                    match action {
                        AppAction::Interaction(keys) => {
                            let actions = vec![];
                            for key in keys {
                                match key {
                                    Mod::Any(Key::MouseMove(x, y)) => {
                                        state.write().await.mouse_pos = (x, y);
                                        state.write().await.need_render = true;
                                    }
                                    Mod::Ctrl(Key::Char('q')) => {
                                        state.write().await.running = false;
                                    }
                                    _ => {}
                                }
                            }
                            if actions.len() > 0 {
                                Some(actions)
                            } else {
                                None
                            }
                        }
                        AppAction::Resize(r) => {
                            state.write().await.need_render = true;
                            None
                        }
                        AppAction::Rendered => {
                            let mut state = state.write().await;
                            state.need_render = false;
                            state.last_render_at = Instant::now();
                            None
                        }
                        _ => None,
                    }
                })
            })
            .await;
        store
            .lock()
            .await
            .effect(
                |state| Cond::becomes_true(state.need_render),
                move |state| {
                    let tui2 = tui2.clone();
                    task::spawn(async move {
                        let app = state.read().await;
                        tui2.lock()
                            .await
                            .terminal
                            .draw(move |frame| app.render(frame))
                            .unwrap();
                        Some(vec![AppAction::Rendered])
                    })
                },
            )
            .await;
        store
            .lock()
            .await
            .effect(
                |state| Cond::becomes_false(state.running),
                move |state| {
                    let b = barrier2.clone();
                    task::spawn(async move {
                        b.wait().await;
                        None
                    })
                },
            )
            .await;

        // Exit the user interface.
        barrier.wait().await;
        let _actions_task = actions_task.await;
        tui.lock().await.exit()?;
        Ok(())
    }

    /// Initializes the terminal interface.
    ///
    /// It enables the raw mode and sets terminal properties.
    pub fn init(&mut self) -> Result<()> {
        terminal::enable_raw_mode()?;
        crossterm::execute!(io::stderr(), EnterAlternateScreen, EnableMouseCapture)?;
        self.terminal.hide_cursor()?;
        self.terminal.clear()?;
        Ok(())
    }

    /// [`Draw`] the terminal interface by [`rendering`] the widgets.
    ///
    /// [`Draw`]: tui::Terminal::draw
    /// [`rendering`]: crate::app::App::render
    /* pub fn draw(&mut self, app: &mut App) -> Result<()> {
           self.terminal.draw(|frame| app.render(frame))?;
           Ok(())
       }
    */
    /// Exits the terminal interface.
    ///
    /// It disables the raw mode and reverts back the terminal properties.
    pub fn exit(&mut self) -> Result<()> {
        terminal::disable_raw_mode()?;
        crossterm::execute!(io::stderr(), LeaveAlternateScreen, DisableMouseCapture)?;
        self.terminal.show_cursor()?;
        Ok(())
    }
}
