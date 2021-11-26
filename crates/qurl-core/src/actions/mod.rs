pub mod Interaction;
use tui::layout::Rect;
use Interaction::Mod;
#[derive(PartialEq, Debug, Clone)]
pub enum AppAction {
    /// An input event occurred.
    Interaction(Vec<Mod>),
    /// Resize event occurred.
    Resize(Rect),
    /// An tick event occurred.
    Tick,
    Rendered,
    Exit,
}
