use anyhow::{anyhow, Result};
use async_std::{
    io::{prelude::BufReadExt, Lines},
    sync::{Arc, Mutex, MutexGuard},
    task,
};
use backtrace::Backtrace;
use clap::{AppSettings, Clap, ErrorKind, ValueHint};
use crossterm::{
    cursor::MoveTo,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    style::Print,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    ExecutableCommand,
};

use qurl_core::{
    state::State,
    ui::util::SMALL_TERMINAL_HEIGHT,
    utils::events::{
        io::IoEvent,
        key::{
            Key,
            Mod::{self, Alt, Any, Clean, Ctrl, Shift},
            Mouse,
        },
        ui::{UiEvent, UiEvents},
    },
};

use async_std::{fs, io::BufReader, path::PathBuf};
use serde_json::{self, json};
use std::{
    borrow::Borrow,
    cell::{Cell, RefCell},
    cmp::{max, min},
    io::{self, stdout},
    panic::{self, PanicInfo},
    sync::mpsc::{self, Receiver, Sender},
    thread,
    time::{self, SystemTime},
};
use syntect::{
    easy::{HighlightFile, HighlightLines},
    highlighting::{Style as SyntStyle, ThemeSet},
    parsing::SyntaxSet,
    util::LinesWithEndings,
};

use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Span, Spans, Text},
    widgets::{Block, BorderType, Borders, Paragraph},
    Terminal,
};

use surf::http::{Method, Url};

#[derive(Clap, Debug)]
#[clap(version = "1.0", author = "Nikolai K.")]
#[clap(setting = AppSettings::ColoredHelp)]
struct Opts {
    /// Sets a custom config file. Could have been an Option<T> with no default too
    // #[clap(short, long, default_value = "default.conf")]
    // config: String,

    /// The URL syntax is protocol-dependent. You'll find a detailed description in RFC 3986.
    #[clap(name = "URL", value_hint = ValueHint::Url)]
    url: String,
    #[clap(short = 'X', long = "method", default_value = "GET")]
    method: Method,
    #[clap(short, long, parse(from_os_str), value_hint = ValueHint::FilePath)]
    output: Option<PathBuf>,
    /// A level of verbosity, and can be used multiple times
    #[clap(short = 'H', long = "header")]
    headers: Vec<String>,

    ///Specify the user name and password to use for server authentication. Overrides -n/--netrc and --netrc-optional.
    #[clap(name = "user:password", short, long)]
    user: Option<String>,
    /// time in ms between two ticks when render ui.
    #[clap(long, default_value = "750")]
    tick_rate: u64,
    /// whether unicode symbols are used to improve the overall look of the app
    #[clap(long)]
    simple_ui: bool,
}

fn close_application() -> Result<()> {
    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture)?;
    Ok(())
}

fn panic_hook(info: &PanicInfo<'_>) {
    if cfg!(debug_assertions) {
        let location = info.location().unwrap();

        let msg = match info.payload().downcast_ref::<&'static str>() {
            Some(s) => *s,
            None => match info.payload().downcast_ref::<String>() {
                Some(s) => &s[..],
                None => "Box<Any>",
            },
        };

        let stacktrace: String = format!("{:?}", Backtrace::new()).replace('\n', "\n\r");

        disable_raw_mode().unwrap();
        execute!(
            io::stdout(),
            LeaveAlternateScreen,
            Print(format!(
                "thread '<unnamed>' panicked at '{}', {}\n\r{}",
                msg, location, stacktrace
            )),
            DisableMouseCapture
        )
        .unwrap();
    }
}

fn main() -> Result<()> {
    panic::set_hook(Box::new(|info| {
        panic_hook(info);
    }));
    //let opts: Opts = Opts::parse();
    let opts: Opts = Opts::parse_from(vec![
        "qurl",
        "http://10.127.5.28:5984/mgn_sws/_all_docs?limit=10",
        "-H",
        "Authorization: Basic YWRtaW46RnljdGggR0hKIQ==",
    ]);

    if opts.tick_rate >= 1000 {
        panic!("Tick rate must be below 1000");
    }
    // Setup input handling

    task::block_on(async {
        let (sync_io_tx, sync_io_rx) = mpsc::channel::<IoEvent>();
        let app = Arc::new(Mutex::new(State::new(sync_io_tx)));

        let cloned_app = Arc::clone(&app);
        // std::thread::spawn(move || {
        //     start_input(sync_io_rx, &app);
        // });
        start_input(sync_io_rx, &app).await;
        // The UI must run in the "main" thread
        start_ui(opts, &cloned_app).await?;
        /* } */

        Ok(())
    })
}

async fn start_input(sync_io_rx: Receiver<IoEvent>, app: &Arc<Mutex<State>>) -> Result<()> {
    let json_txt = fs::read_to_string("assets/egko_subway.json").await.unwrap();
    let json_obj =
        serde_json::from_str(&json_txt[..]).unwrap_or(json!({"__ERROR__":"Failed to parse input"}));
    let json_txt_pretty = serde_json::to_string_pretty(&json_obj).unwrap();
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];
    let syntax = ps.find_syntax_by_extension("json").unwrap();
    let mut h = HighlightLines::new(syntax, theme);
    let mut l = 0;
    for line in LinesWithEndings::from(&json_txt_pretty[..]) {
        l += 1;
        let ranges: Vec<(SyntStyle, &str)> = h.highlight(line, &ps);
        let mut spans: Vec<Span<'static>> = vec![Span::styled(
            format!("{:^6} │ ", l),
            Style::default() /* .bg(Color::Black) */
                .fg(Color::DarkGray),
        )];
        for &(ref style, text) in ranges.iter() {
            spans.push(Span::styled(
                text.replace(" ", "·")
                    .replace("\t", "⇥   ")
                    .replace(" ", "·")
                    .replace("\r", "␍")
                    .replace("\n", "␊"),
                Style::default()
                    /* .bg(Color::Rgb(
                        style.background.r,
                        style.background.g,
                        style.background.b,
                    )) */
                    .fg(Color::Rgb(
                        style.foreground.r,
                        style.foreground.g,
                        style.foreground.b,
                    )),
            ));
        }

        let mut app = app.lock().await;
        app.input_text.push(Spans::from(spans.to_owned()));
        // if (l >= 70) {
        //     break;
        // }
        //print!("\n{:^6}│{}", l,escaped);
    }
    Ok(())
}
async fn start_ui(opts: Opts, app: &Arc<Mutex<State>>) -> Result<()> {
    // Terminal initialization
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = UiEvents::new(opts.tick_rate);

    // play music on, if not send them to the device selection view

    let mut is_first_render = true;

    let mut mouse_pos = (0u16, 0u16);
    if let Ok(size) = terminal.backend().size() {
        let mut app = app.lock().await;
        app.on_resize(size);
    };
    let delay = time::Duration::from_millis(33);
    let mut now = time::Instant::now();
    let mut frame_time = now.elapsed();
    let mut _events: Vec<Mod> = vec![];
    ///////////////
    /////////////////
    /* let (text_tx, text_rx) = mpsc::channel();
    std::thread::spawn(move || task::block_on(async { start_input(text_tx).await })); */
    'main: loop {
        let mut app = app.lock().await;
        if (now.elapsed() < delay) {
            thread::sleep(delay - now.elapsed());
        }
        /* let current_route = app.get_current_route(); */
        //if (_events.len() >= 0) {

        terminal.draw(|mut f| {
            let block = Block::default()
                .borders(Borders::ALL)
                .title(Span::styled(
                    format!(
                        " Main block with round corners {{frame_time: {:?}, lines: {:?}}} ",
                        now.elapsed(),
                        app.input_text.len(),
                    )
                    .to_string(),
                    Style::default().fg(Color::White),
                ))
                .title_alignment(Alignment::Center)
                .border_type(BorderType::Plain)
                .border_style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .bg(Color::Rgb(0x2b, 0x30, 0x3b)),
                );

            //f.render_widget(block, app.size);
            let text = Text {
                lines: app.input_text.clone(),
            };
            let paragraph = Paragraph::new(text)
                .block(block)
                .style(Style::default().bg(Color::Rgb(0x2b, 0x30, 0x3b)));
            f.render_widget(paragraph, app.size);
            /* match current_route.active_block {
            ActiveBlock::HelpMenu => {
                ui::draw_help_menu(&mut f, &app);
            }
            ActiveBlock::Error => {
                ui::draw_error_screen(&mut f, &app);
            }
            ActiveBlock::SelectDevice => {
                ui::draw_device_list(&mut f, &app);
            }
            ActiveBlock::Analysis => {
                ui::audio_analysis::draw(&mut f, &app);
            }
            ActiveBlock::BasicView => {
                ui::draw_basic_view(&mut f, &app);
            }
            _ => {
                ui::draw_main_layout(&mut f, &app);
            } */
        })?;
        //}
        now = time::Instant::now();
        /* if current_route.active_block == ActiveBlock::Input {
            terminal.show_cursor()?;
        } else {
            terminal.hide_cursor()?;
        }

         */
        let cursor_offset = if app.size.height > SMALL_TERMINAL_HEIGHT {
            2
        } else {
            1
        };

        // Put the cursor back inside the input box
        terminal.backend_mut().execute(MoveTo(
            cursor_offset + app.input_cursor_position,
            cursor_offset,
        ))?;

        // Handle authentication refresh
        // if SystemTime::now() > app.spotify_token_expiry {
        //     app.dispatch(IoEvent::RefreshAuthentication);
        // }
        _events = vec![];
        loop {
            match events.next()? {
                UiEvent::Resize(size) => {
                    if (app.size != size) {
                        app.on_resize(size);
                        _events.push(Mod::Clean(Key::Unknown))
                    }
                }
                UiEvent::Input(key) => {
                    _events.push(key);
                    match key {
                        Ctrl(Key::Char('q')) => break 'main,
                        /* Any(Key::MouseDown(Mouse::Left, x, y)) => {
                            mpos = (x, y);
                        }, */
                        Any(Key::MouseMove(x, y)) => {
                            mouse_pos = (x, y);
                        }
                        _ => {}
                    }

                    /*  let current_active_block = app.get_current_route().active_block;

                    // To avoid swallowing the global key presses `q` and `-` make a special
                    // case for the input handler
                    if current_active_block == ActiveBlock::Input {
                        handlers::input_handler(key, &mut app);
                    } else if key == app.user_config.keys.back {
                        if app.get_current_route().active_block != ActiveBlock::Input {
                            // Go back through navigation stack when not in search input mode and exit the app if there are no more places to back to

                            let pop_result = match app.pop_navigation_stack() {
                                Some(ref x) if x.id == RouteId::Search => app.pop_navigation_stack(),
                                Some(x) => Some(x),
                                None => None,
                            };
                            if pop_result.is_none() {
                                break; // Exit application
                            }
                        }
                    } else {
                        handlers::handle_app(key, &mut app);
                    } */
                }
                UiEvent::Tick => {
                    app.on_tick();
                }
            }

            if now.elapsed() >= delay {
                break;
            }
        }
        if (_events.len() > 0) {
            app.on_input(&_events);
        }
        // Delay spotify request until first render, will have the effect of improving
        // startup speed
        /* if is_first_render {
            app.dispatch(IoEvent::GetPlaylists);
            app.dispatch(IoEvent::GetUser);
            app.dispatch(IoEvent::GetCurrentPlayback);
            app.help_docs_size = ui::help::get_help_docs(&app.user_config.keys).len() as u32;

            is_first_render = false;
        } */
    }
    terminal.show_cursor()?;
    close_application()?;

    Ok(())
}
