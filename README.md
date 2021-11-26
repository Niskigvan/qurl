% qurl

Like curl, but with interactive data processing with `jq`[^jq][^jaq] for JSON/JSON-SEQ/XML/YAML/CSV/XLSX and `fuzzy` search with `skim`[^skim] (like `fzf`[^fzf]).

[^jq]: https://stedolan.github.io/jq/
[^jaq]: https://github.com/01mf02/jaq
[^fzf]: https://github.com/junegunn/fz
[^skim]: https://github.com/lotabout/skim#color-scheme
[^syntect]: https://github.com/trishume/syntect
# Installation

To use qurl, you need a Rust toolchain.
See <https://rustup.rs/> for instructions.
(Note that Rust compilers shipped with Linux distributions
may be too outdated to compile qurl. I use Rust 1.54.)

And the latest development version:

    cargo install --branch main --git https://github.com/Niskigvan/qurl

qurl should work on any system supported by Rust.
If it does not, please file an issue.

## Features

- support JSON/JSON-SEQ/XML/YAML/CSV/XLSX as input and output [WIP]
- Full UTF-8 support [WIP]
- JQ autocompletion [WIP]
- Syntax highlighting for input/output/jq powered by syntect[^syntect] [WIP]
- work on shell (Windows/Mac/Linux/Redox) and browser (Wasm) [WIP]