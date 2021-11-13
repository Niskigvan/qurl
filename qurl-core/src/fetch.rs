#![warn(rust_2018_idioms)]
use clap::{AppSettings, Clap, ErrorKind, ValueHint};
use futures_lite::stream::StreamExt;
use reqwest::{
    header::{HeaderMap, HeaderValue},
    Client, Method, Request, ResponseBuilderExt,
};
use std::path::PathBuf;
use syntect::easy::{HighlightFile, HighlightLines};
use syntect::highlighting::{Style, ThemeSet};
use syntect::parsing::SyntaxSet;
use syntect::util::{as_24_bit_terminal_escaped, LinesWithEndings};

use serde_json::json;

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
}

//#[tokio::main]

fn main() -> Result<(), std::io::Error> {
    let opts = Opts::parse_from(vec![
        "qurl",
        "http://10.127.5.28:5984/mgn_sws/_all_docs?limit=10",
        "-H",
        "Authorization: Basic YWRtaW46RnljdGggR0hKIQ==",
    ]);
    let mut req = reqwest::blocking::Client::new().request(opts.method, opts.url);
    if let Some(user) = opts.user {
        let split = user.trim().split(':').collect::<Vec<&str>>();
        req = req.basic_auth(split.get(0).unwrap(), split.get(1));
    }

    if opts.headers.len() > 0 {
        let mut headers = HeaderMap::new();
        for h in opts.headers.iter() {
            let mut split = h.trim().split(':');
            let (k, v) = (split.next().unwrap(), split.next().unwrap_or(""));
            let k = unsafe {
                let slice = std::slice::from_raw_parts(k.as_ptr(), k.len());
                std::str::from_utf8(slice).unwrap()
            };
            let v = unsafe {
                let slice = std::slice::from_raw_parts(v.as_ptr(), v.len());
                std::str::from_utf8(slice).unwrap()
            };
            headers.insert(k, HeaderValue::from_str(v).unwrap());
        }
        req = req.headers(headers);
    }
    let res = req.send().unwrap();
    let res_txt = res.text().unwrap();
    let json_obj =
        serde_json::from_str(&res_txt[..]).unwrap_or(json!({"__ERROR__":"Failed to parse input"}));

    let json_txt = serde_json::to_string_pretty(&json_obj).unwrap();

    // let stdout = io::stdout();
    // let backend = CrosstermBackend::new(stdout);
    // let mut terminal = Terminal::new(backend)?;
    // terminal.draw(|f| {
    //     let size = f.size();
    //     let block = Block::default()
    //         .title("Block")
    //         .borders(Borders::ALL);
    //     f.render_widget(block, size);
    // })?;

    let ps = SyntaxSet::load_defaults_newlines();
    let ts = ThemeSet::load_defaults();
    let theme = &ts.themes["base16-ocean.dark"];
    let syntax = ps.find_syntax_by_extension("json").unwrap();
    let mut h = HighlightLines::new(syntax, theme);
    let mut l = 0;
    for line in LinesWithEndings::from(&json_txt[..]) {
        let ranges: Vec<(Style, &str)> = h.highlight(line, &ps);
        let escaped = as_24_bit_terminal_escaped(&ranges[..], true)
            .replace(" ", "·")
            .replace("\t", "⇥   ")
            .replace(" ", "·")
            .replace("\r", "␍")
            .replace("\n", "␊");
        //println!("\n{:?}", line);
        l += 1;
        println!("{}", escaped);
        //print!("\n{:^6}│{}", l,escaped);
    }

    Ok(())
}
