use crossterm::event;
use std::fmt;

#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Mouse {
    Left,
    Right,
    Middle,
}
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Key {
    /// Both Enter (or Return) and numpad Enter
    Enter,
    /// Tabulation key
    Tab,
    /// Backspace key
    Backspace,
    /// Escape key
    Esc,

    /// Left arrow
    Left,
    /// Right arrow
    Right,
    /// Up arrow
    Up,
    /// Down arrow
    Down,

    /// Insert key
    Ins,
    /// Delete key
    Delete,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,

    /// F0 key
    F0,
    /// F1 key
    F1,
    /// F2 key
    F2,
    /// F3 key
    F3,
    /// F4 key
    F4,
    /// F5 key
    F5,
    /// F6 key
    F6,
    /// F7 key
    F7,
    /// F8 key
    F8,
    /// F9 key
    F9,
    /// F10 key
    F10,
    /// F11 key
    F11,
    /// F12 key
    F12,
    /// Char key. Contains char event occurred on.
    Char(char),
    /// Moved the mouse cursor while not pressing a mouse button. Contains row,column event occurred on.
    MouseMove(u16, u16),
    /// Scrolled mouse wheel downwards (towards the user). Contains row,column,dir event occurred on.
    MouseScroll(i32, u16, u16),
    /// Pressed Left mouse button. Contains row,column event occurred on.
    MouseDown(Mouse, u16, u16),
    /// Released Left mouse button. Contains row,column event occurred on.
    MouseUp(Mouse, u16, u16),
    /// Moved the mouse cursor while pressing Left mouse button. Contains row,column event occurred on.
    MouseDrag(Mouse, u16, u16),
    MouseClick(Mouse, u16, u16),
    Unknown,
}

/// Represents an key Modifier.
#[derive(PartialEq, Eq, Clone, Copy, Hash, Debug)]
pub enum Mod {
    Ctrl(Key),
    Alt(Key),
    Shift(Key),
    Any(Key),
    Clean(Key),
}
///////////////////////////////////////////////////////////////////////////////////////////
impl Key {
    /// Returns the function key corresponding to the given number
    ///
    /// 1 -> F1, etc...
    ///
    /// # Panics
    ///
    /// If `n == 0 || n > 12`
    pub fn from_f(n: u8) -> Key {
        match n {
            0 => Key::F0,
            1 => Key::F1,
            2 => Key::F2,
            3 => Key::F3,
            4 => Key::F4,
            5 => Key::F5,
            6 => Key::F6,
            7 => Key::F7,
            8 => Key::F8,
            9 => Key::F9,
            10 => Key::F10,
            11 => Key::F11,
            12 => Key::F12,
            _ => panic!("unknown function key: F{}", n),
        }
    }
}

impl fmt::Display for Key {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Key::Char(' ') => write!(f, "<Space>"),
            Key::Char(c) => write!(f, "{}", c),
            Key::Left | Key::Right | Key::Up | Key::Down => write!(f, "<Arrow({:?})>", self),
            Key::Enter
            | Key::Tab
            | Key::Backspace
            | Key::Esc
            | Key::Ins
            | Key::Delete
            | Key::Home
            | Key::End
            | Key::PageUp
            | Key::PageDown => write!(f, "<{:?}>", self),
            Key::MouseMove(..)
            | Key::MouseScroll(..)
            | Key::MouseDown(..)
            | Key::MouseUp(..)
            | Key::MouseDrag(..)
            | Key::MouseClick(..) => write!(f, "<{:?}>", self),
            _ => write!(f, "{:?}", self),
        }
    }
}
impl fmt::Display for Mod {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Mod::Alt(key) => match key {
                Key::Char(' ') => write!(f, "<Alt+Space>"),
                Key::Char(c) => write!(f, "<Alt+{:?}>", c),
                Key::Left | Key::Right | Key::Up | Key::Down => {
                    write!(f, "<Alt+ArrowKey({:?})>", key)
                }
                _ => write!(f, "<Alt+{:?}>", key),
            },
            Mod::Ctrl(key) => match key {
                Key::Char(' ') => write!(f, "<Ctrl+Space>"),
                Key::Char(c) => write!(f, "<Ctrl+{:?}>", c),
                Key::Left | Key::Right | Key::Up | Key::Down => {
                    write!(f, "<Ctrl+Arrow({:?})>", key)
                }
                _ => write!(f, "<Ctrl+Arrow({:?})>", key),
            },
            Mod::Shift(key) => match key {
                Key::Char(' ') => write!(f, "<Shift+Space>"),
                Key::Left | Key::Right | Key::Up | Key::Down => {
                    write!(f, "<Shift+Arrow({:?})>", key)
                }
                _ => write!(f, "<Shift+{:?})>", key),
            },
            _ => write!(f, "{:?}", self),
        }
    }
}
impl From<event::KeyCode> for Key {
    fn from(code: event::KeyCode) -> Self {
        match code {
            event::KeyCode::Esc => Key::Esc,
            event::KeyCode::Backspace => Key::Backspace,
            event::KeyCode::Left => Key::Left,
            event::KeyCode::Right => Key::Right,
            event::KeyCode::Up => Key::Up,
            event::KeyCode::Down => Key::Down,
            event::KeyCode::Home => Key::Home,
            event::KeyCode::End => Key::End,
            event::KeyCode::PageUp => Key::PageUp,
            event::KeyCode::PageDown => Key::PageDown,
            event::KeyCode::Delete => Key::Delete,
            event::KeyCode::Insert => Key::Ins,
            event::KeyCode::F(n) => Key::from_f(n),
            event::KeyCode::Char(c) => Key::Char(c),
            event::KeyCode::Enter => Key::Enter,
            event::KeyCode::Tab => Key::Tab,
            _ => Key::Unknown,
        }
    }
}
impl From<event::KeyEvent> for Key {
    fn from(key_event: event::KeyEvent) -> Self {
        match key_event {
            event::KeyEvent { code, .. } => Key::from(code),
            _ => Key::Unknown,
        }
    }
}
impl From<event::MouseButton> for Mouse {
    fn from(btn: event::MouseButton) -> Self {
        match btn {
            event::MouseButton::Left => Mouse::Left,
            event::MouseButton::Right => Mouse::Right,
            event::MouseButton::Middle => Mouse::Middle,
        }
    }
}
impl From<event::MouseEvent> for Key {
    fn from(mouse_event: event::MouseEvent) -> Self {
        match mouse_event {
            event::MouseEvent {
                column, row, kind, ..
            } => match kind {
                event::MouseEventKind::Down(btn) => Key::MouseDown(Mouse::from(btn), column, row),
                event::MouseEventKind::Up(btn) => Key::MouseUp(Mouse::from(btn), column, row),
                event::MouseEventKind::Drag(btn) => Key::MouseDrag(Mouse::from(btn), column, row),
                event::MouseEventKind::Moved => Key::MouseMove(column, row),
                event::MouseEventKind::ScrollUp => Key::MouseScroll(-1, column, row),
                event::MouseEventKind::ScrollDown => Key::MouseScroll(1, column, row),
                _ => Key::Unknown,
            },
            _ => Key::Unknown,
        }
    }
}
impl From<event::KeyEvent> for Mod {
    fn from(key_event: event::KeyEvent) -> Self {
        match key_event {
            event::KeyEvent {
                code,
                modifiers: event::KeyModifiers::ALT,
            } => Mod::Alt(Key::from(code)),
            event::KeyEvent {
                code,
                modifiers: event::KeyModifiers::CONTROL,
            } => Mod::Ctrl(Key::from(code)),
            event::KeyEvent {
                code,
                modifiers: event::KeyModifiers::SHIFT,
            } => match code {
                event::KeyCode::Char(_) => Mod::Clean(Key::from(code)),
                _ => Mod::Shift(Key::from(code)),
            },
            _ => Mod::Clean(Key::from(key_event.code)),
        }
    }
}
impl From<event::MouseEvent> for Mod {
    fn from(mouse_event: event::MouseEvent) -> Self {
        match mouse_event {
            event::MouseEvent {
                modifiers: event::KeyModifiers::ALT,
                ..
            } => Mod::Alt(Key::from(mouse_event)),
            event::MouseEvent {
                modifiers: event::KeyModifiers::CONTROL,
                ..
            } => Mod::Ctrl(Key::from(mouse_event)),
            event::MouseEvent {
                modifiers: event::KeyModifiers::SHIFT,
                ..
            } => Mod::Shift(Key::from(mouse_event)),
            _ => Mod::Clean(Key::from(mouse_event)),
        }
    }
}
