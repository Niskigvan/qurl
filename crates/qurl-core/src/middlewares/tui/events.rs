use crate::{
    actions::{AppAction, Interaction::*},
    AppStore,
};
use anyhow::Result;
use async_std::{stream::StreamExt, task};
use async_store::ArcStore;
use crossterm::event;
use futures::{select, FutureExt};
use futures_timer::Delay;
use std::time::{Duration, Instant};
use tui::layout::Rect;

pub struct InteractionStream {
    reader: event::EventStream,
    store: AppStore,
}
impl InteractionStream {
    /// Constructs an new instance of `Events` with the default config.
    pub fn new(store: AppStore) -> InteractionStream {
        let reader = event::EventStream::new();
        InteractionStream { reader, store }
    }
    /// Attempts to read an event.
    /// This function will block the current thread.
    pub async fn next(&mut self) -> () {
        let mut key_events = vec![];
        let mut mouse_events = vec![];
        let mut actions = vec![];
        let mut tick_rate;
        let mut frame_rate;
        let mut last_render_at;
        {
            let app = self.store.lock().await;
            let s = app.state.read().await;
            tick_rate = s.options.tick_rate.clone();
            frame_rate = s.options.frame_rate.clone();
            last_render_at = s.last_render_at.clone();
        }

        loop {
            let mut signal = false;
            select! {
                _ = Delay::new(tick_rate).fuse()  => {
                    signal=true
                },
                maybe_event = self.reader.next().fuse() => {
                    if signal {
                        return;
                    }
                    match maybe_event {
                        Some(Ok(event)) => match event {
                            event::Event::Key(key_event) => {
                                key_events.push(Mod::from(key_event));
                                let key = Key::from(key_event.code);
                                if let Key::Char(key_char) = key {
                                    let ch = char::to_uppercase(key_char).collect::<Vec<char>>()[0];
                                    if ch == key_char {
                                        let ch = char::to_lowercase(key_char).collect::<Vec<char>>()[0];
                                        key_events.push(Mod::Any(Key::Char(ch)));
                                    }
                                }
                                key_events.push(Mod::Any(key));
                            }
                            event::Event::Mouse(mouse_event) => {
                                mouse_events = vec![Mod::from(mouse_event), Mod::Any(Key::from(mouse_event))];
                            }
                            event::Event::Resize(width, height) => {
                                actions=vec![AppAction::Resize(Rect {
                                    width,
                                    height,
                                    ..Default::default()
                                })];
                            }
                        },
                        _=>{}
                        /* Some(Err(e)) =>,
                        None => {}, */
                    }
                }
            };
            //Delay::new(Duration::from_millis(1)).await;
            {
                let app = self.store.lock().await;
                let s = app.state.read().await;
                last_render_at = s.last_render_at.clone();
            }
            if last_render_at.elapsed() >= frame_rate {
                break;
            }
        }
        key_events.append(&mut mouse_events);
        actions.push(AppAction::Interaction(key_events));
        for action in actions {
            self.store.dispatch(action).await
        }
    }
    pub async fn run(app: AppStore) -> Result<()> {
        let mut events_ = Self::new(app.clone());
        loop {
            events_.next().await;
            if !app.lock().await.state.read().await.running {
                break;
            }
        }
        Ok(())
    }
}
