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
    state::Glob,
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
use rayon::iter::IndexedParallelIterator;
use serde_json::{self, json};
use std::{borrow::Borrow, cell::{Cell, RefCell}, cmp::{max, min}, io::{self, stdout}, panic::{self, PanicInfo}, sync::mpsc::{self, Receiver, Sender}, thread, time::{self, Duration, SystemTime}};
use syntect::{dumps::from_binary, easy::{HighlightFile, HighlightLines}, highlighting::{Style as SyntStyle, ThemeSet}, parsing::SyntaxSet, util::LinesWithEndings};

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
    #[clap(long, default_value = "160")]
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
            stdout(),
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

    execute!(stdout(), EnterAlternateScreen, EnableMouseCapture)?;
    enable_raw_mode()?;
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
        let app = Arc::new(Mutex::new(Glob::new(sync_io_tx)));

        let cloned_app = Arc::clone(&app);
        // std::thread::spawn(move || {
        //     start_input(sync_io_rx, &app);
        // });
        {
            app.lock().await.jq_input=". | contains(\"never\")".to_string();
        }
        
        thread::spawn(move || task::block_on(start_input(sync_io_rx, &app)));
        // The UI must run in the "main" thread
        start_ui(opts, &cloned_app).await?;
        /* } */

        Ok(())
    })
}

async fn start_input(sync_io_rx: Receiver<IoEvent>, app: &Arc<Mutex<Glob>>) -> Result<()> {
    
    let json_txt = fs::read_to_string(
        format!("{}/../testdata/egko_subway.json", env!("CARGO_MANIFEST_DIR"))).await.unwrap();
    let json_obj =
        serde_json::from_str(&json_txt[..]).unwrap_or(json!({"__ERROR__":"Failed to parse input"}));
    let json_txt_pretty = serde_json::to_string_pretty(&json_obj).unwrap();
    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];
    let syntax = ps.find_syntax_by_extension("json").unwrap();
    let mut h = HighlightLines::new(syntax, theme);
    for line in LinesWithEndings::from(&json_txt_pretty[..]) {
        let mut ranges: Vec<Vec<(SyntStyle, String)>> = vec![h.highlight(line, &ps)
        .into_iter().map(|v|(v.0,
            v.1.replace(" ", "·")
                .replace("\t", "⇥   ")
                .replace(" ", "·")
                .replace("\r", "␍")
                .replace("\n", "␊").to_owned(),
        )).collect()];

        let mut app = app.lock().await;
        app.inp_data
            .formatted_lines
            .append(&mut ranges);
        // if (l >= 70) {
        //     break;
        // }
        //print!("\n{:^6}│{}", l,escaped);
    }
    {
        
        let mut app = app.lock().await;
        app.inp_data.formatted_loaded=true;
        app.inp_data.original_loaded=true;
    }
    task::sleep(Duration::from_millis(250)).await;
    {
        let mut app = app.lock().await;
        app.pause=true;
    }
    
    Ok(())
}
async fn start_ui(opts: Opts, app: &Arc<Mutex<Glob>>) -> Result<()> {
    // Terminal initialization

    let backend = CrosstermBackend::new(stdout());
    let mut terminal = Terminal::new(backend)?;
    terminal.hide_cursor()?;

    let events = UiEvents::new(opts.tick_rate);

    // play music on, if not send them to the device selection view

    let mut mouse_pos = (0u16, 0u16);
    if let Ok(size) = terminal.backend().size() {
        let mut app = app.lock().await;
        app.on_resize(size);
    };
    let delay = time::Duration::from_millis(opts.tick_rate/10u64);
    let mut now = time::Instant::now();
    let mut frame_time = now.elapsed();
    let mut _events: Vec<Mod> = vec![];
    let mut loader_i=0;
    let loader_glyph="⣾⣽⣻⢿⡿⣟⣯⣷".to_string();
    //let wait=
    ///////////////
    /////////////////
    /* let (text_tx, text_rx) = mpsc::channel();
    std::thread::spawn(move || task::block_on(async { start_input(text_tx).await })); */
    'main: loop {
        if (now.elapsed() < delay) {
            thread::sleep(delay - now.elapsed());
        }

        now = time::Instant::now();
        
        let mut pause=true;
        {
            let app=app.lock().await;
            pause=app.pause
        }
        
        /* let current_route = app.get_current_route(); */
        if _events.len() > 0 || !pause{
            let app = app.lock().await;
            terminal.draw(|mut f| {
                loader_i=(loader_i+1)%8;
                let cont = &app.inp_data;
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(0)
                    .constraints([ Constraint::Length(3),Constraint::Percentage(50)].as_ref())
                    .split(f.size());
                let b_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(50),Constraint::Percentage(50)].as_ref())
                    .split(chunks[1]);
                let l_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .margin(1)
                    .constraints([Constraint::Length(6),Constraint::Percentage(100)].as_ref())
                    .split(b_chunks[0]);

                /////////////
                let lines = &cont.formatted_lines;
                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        format!(
                            " [{}] INPUT {{frame_time: {:?}, lines: {:?}, scroll: {:?},mouse_pos: {:?}}} ",
                            if cont.formatted_loaded {"+".chars().next().unwrap()}
                            else {loader_glyph.chars().nth(loader_i).unwrap_or(loader_glyph.chars().next().unwrap())},
                            now.elapsed(),
                            lines.len(),
                            cont.scroll,mouse_pos
                        )
                        .to_string(),
                        Style::default().fg(Color::White),
                    ))
                    .title_alignment(Alignment::Left)
                    .border_type(BorderType::Plain)
                    .border_style(
                        Style::default()
                            .fg(Color::DarkGray)
                            /* .bg(Color::Rgb(0x2b, 0x30, 0x3b)) */,
                    );
                
                f.render_widget(block, b_chunks[0]);

                let paragraph = Paragraph::new(Text{
                    lines: ((cont.scroll.0)..(cont.scroll.0+(f.size().height-5)))
                    .map(|n|Spans::from(n.to_string())).collect()
                })
                    .style(Style::default()/* .bg(Color::Rgb(0x2b, 0x30, 0x3b)) */);
                f.render_widget(paragraph, l_chunks[0]);

                
                let text = Text {
                    lines:lines.iter()
                        .skip(cont.scroll.0 as usize).take((f.size().height-5)  as usize)
                        .map(|v| Spans(v.iter().map(|v|Span {
                            content: v.1.clone().into(),
                            style: Style::default()
                            /* .bg(Color::Rgb(
                                style.background.r,
                                style.background.g,
                                style.background.b,
                            )) */
                            .fg(Color::Rgb(
                                v.0.foreground.r,
                                v.0.foreground.g,
                                v.0.foreground.b,
                            )),
                            }
                            
                            ).collect())
                        ).collect()
                };
                let paragraph = Paragraph::new(text)
                    .scroll((0,cont.scroll.1))
                    .style(Style::default()/* .bg(Color::Rgb(0x2b, 0x30, 0x3b)) */);
                f.render_widget(paragraph, l_chunks[1]);

                let block = Block::default()
                    .borders(Borders::ALL)
                    .title(Span::styled(
                        "Query".to_string(),
                        Style::default().fg(Color::White),
                    ))
                    .title_alignment(Alignment::Left)
                    .border_type(BorderType::Plain)
                    .border_style(
                        Style::default()
                            .fg(Color::DarkGray)
                            /* .bg(Color::Rgb(0x2b, 0x30, 0x3b)) */,
                    );
                let paragraph = Paragraph::new(Text::from(app.jq_input.clone()))
                        .block(block)
                        .style(Style::default()/* .bg(Color::Rgb(0x2b, 0x30, 0x3b)) */);
                f.render_widget(paragraph, chunks[0]);
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
        }

        /* if current_route.active_block == ActiveBlock::Input {
            terminal.show_cursor()?;
        } else {
            terminal.hide_cursor()?;
        }

         */

        // Handle authentication refresh
        // if SystemTime::now() > app.spotify_token_expiry {
        //     app.dispatch(IoEvent::RefreshAuthentication);
        // }
        _events = vec![];
        loop {
            match events.next()? {
                UiEvent::Resize(_) => {
                    let mut app = app.lock().await;
                    let size = terminal.backend().size()?;
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

                        // Clean(Key::MouseScroll(..))
                        // | Alt(Key::MouseScroll(..))
                        // | Shift(Key::MouseScroll(..))
                        // | Ctrl(Key::MouseScroll(..))
                        // | Any(Key::MouseScroll(..)) => {
                        //     if _events
                        //         .iter()
                        //         .filter(|ev| match ev {
                        //             Any(Key::MouseScroll(..)) => true,
                        //             _ => false,
                        //         })
                        //         .collect::<Vec<&Mod>>()
                        //         .len()
                        //         < 5
                        //     {
                        //         _events.push(key);
                        //     }
                        // }
                        _ => {
                            _events.push(key);
                        }
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
                    let mut app = app.lock().await;
                    app.on_tick();
                }
            }

            if now.elapsed() >= delay {
                break;
            }
        }
        if _events.len() > 0 {
            let mut app = app.lock().await;
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
