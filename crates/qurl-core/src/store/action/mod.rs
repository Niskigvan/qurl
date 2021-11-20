pub mod Interaction;
use tui::layout::Rect;
use Interaction::Mod;
pub enum AppAction {
    /// An input event occurred.
    Interaction(Vec<Mod>),
    /// An tick event occurred.
    Tick,
    /// Resize event occurred.
    Resize(Rect),
}
