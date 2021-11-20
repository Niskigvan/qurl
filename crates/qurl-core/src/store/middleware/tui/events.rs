use crossterm::event;
use std::{
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};
use tui::layout::Rect;

pub struct TermEventsHandler {
    pub tick_rate: Duration,
    rx: mpsc::Receiver<UiEvent<Mod>>,
    // Need to be kept around to prevent disposing the sender side.
    _tx: mpsc::Sender<UiEvent<Mod>>,
}

impl TermEventsHandler {
    /// Constructs an new instance of `Events` with the default config.
    pub fn new(tick_rate: u64) -> TermEventsHandler {
        let tick_rate = Duration::from_millis(tick_rate);
        let (tx, rx) = mpsc::channel();
        let event_tx = tx.clone();
        thread::spawn(move || {
            loop {
                // poll for tick rate duration, if no event, sent tick event.
                if event::poll(tick_rate).unwrap() {
                    match event::read().unwrap() {
                        event::Event::Key(key_event) => {
                            let key_mod = Mod::from(key_event);
                            event_tx.send(UiEvent::Input(key_mod)).unwrap();
                            let key = Key::from(key_event.code);
                            if let Key::Char(key_char) = key {
                                let ch = char::to_uppercase(key_char).collect::<Vec<char>>()[0];
                                if ch == key_char {
                                    let ch = char::to_lowercase(key_char).collect::<Vec<char>>()[0];
                                    event_tx
                                        .send(UiEvent::Input(Mod::Any(Key::Char(ch))))
                                        .unwrap();
                                }
                            }
                            event_tx.send(UiEvent::Input(Mod::Any(key))).unwrap();
                        }
                        event::Event::Mouse(mouse_event) => {
                            let mouse_mod = Mod::from(mouse_event);
                            event_tx.send(UiEvent::Input(mouse_mod)).unwrap();
                            let key = Key::from(mouse_event);
                            event_tx.send(UiEvent::Input(Mod::Any(key))).unwrap();
                        }
                        event::Event::Resize(width, height) => {
                            event_tx
                                .send(UiEvent::Resize(Rect {
                                    width,
                                    height,
                                    ..Default::default()
                                }))
                                .unwrap();
                        }
                    }
                }

                event_tx.send(UiEvent::Tick).unwrap();
            }
        });

        TermEventsHandler {
            rx,
            _tx: tx,
            tick_rate,
        }
    }
    /// Attempts to read an event.
    /// This function will block the current thread.
    pub fn next(&self) -> Result<UiEvent<Mod>, mpsc::RecvError> {
        self.rx.recv()
    }
}
impl Default for TermEventsHandler {
    fn default() -> Self {
        TermEventsHandler::new(250)
    }
}
