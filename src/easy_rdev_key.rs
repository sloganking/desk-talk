/// This is a copy of rdev::Key, so that #[derive(clap::ValueEnum)] works.
///
/// I also added F13 through F24 for convenience.
#[derive(clap::ValueEnum, Clone, Copy, Debug)]
pub enum PTTKey {
    /// Alt key on Linux and Windows (option key on macOS)
    Alt,
    AltGr,
    Backspace,
    CapsLock,
    ControlLeft,
    ControlRight,
    Delete,
    DownArrow,
    End,
    Escape,
    F1,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    Home,
    LeftArrow,
    /// also known as "windows", "super", and "command"
    MetaLeft,
    /// also known as "windows", "super", and "command"
    MetaRight,
    PageDown,
    PageUp,
    Return,
    RightArrow,
    ShiftLeft,
    ShiftRight,
    Space,
    Tab,
    UpArrow,
    PrintScreen,
    ScrollLock,
    Pause,
    NumLock,
    BackQuote,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Num0,
    Minus,
    Equal,
    KeyQ,
    KeyW,
    KeyE,
    KeyR,
    KeyT,
    KeyY,
    KeyU,
    KeyI,
    KeyO,
    KeyP,
    LeftBracket,
    RightBracket,
    KeyA,
    KeyS,
    KeyD,
    KeyF,
    KeyG,
    KeyH,
    KeyJ,
    KeyK,
    KeyL,
    SemiColon,
    Quote,
    BackSlash,
    IntlBackslash,
    KeyZ,
    KeyX,
    KeyC,
    KeyV,
    KeyB,
    KeyN,
    KeyM,
    Comma,
    Dot,
    Slash,
    Insert,
    KpReturn,
    KpMinus,
    KpPlus,
    KpMultiply,
    KpDivide,
    Kp0,
    Kp1,
    Kp2,
    Kp3,
    Kp4,
    Kp5,
    Kp6,
    Kp7,
    Kp8,
    Kp9,
    KpDelete,
    Function,
    #[clap(skip)]
    Unknown(u32),
}

impl From<PTTKey> for rdev::Key {
    fn from(item: PTTKey) -> Self {
        match item {
            PTTKey::Alt => rdev::Key::Alt,
            PTTKey::AltGr => rdev::Key::AltGr,
            PTTKey::Backspace => rdev::Key::Backspace,
            PTTKey::CapsLock => rdev::Key::CapsLock,
            PTTKey::ControlLeft => rdev::Key::ControlLeft,
            PTTKey::ControlRight => rdev::Key::ControlRight,
            PTTKey::Delete => rdev::Key::Delete,
            PTTKey::DownArrow => rdev::Key::DownArrow,
            PTTKey::End => rdev::Key::End,
            PTTKey::Escape => rdev::Key::Escape,
            PTTKey::F1 => rdev::Key::F1,
            PTTKey::F10 => rdev::Key::F10,
            PTTKey::F11 => rdev::Key::F11,
            PTTKey::F12 => rdev::Key::F12,
            PTTKey::F13 => rdev::Key::Unknown(124),
            PTTKey::F14 => rdev::Key::Unknown(125),
            PTTKey::F15 => rdev::Key::Unknown(126),
            PTTKey::F16 => rdev::Key::Unknown(127),
            PTTKey::F17 => rdev::Key::Unknown(128),
            PTTKey::F18 => rdev::Key::Unknown(129),
            PTTKey::F19 => rdev::Key::Unknown(130),
            PTTKey::F20 => rdev::Key::Unknown(131),
            PTTKey::F21 => rdev::Key::Unknown(132),
            PTTKey::F22 => rdev::Key::Unknown(133),
            PTTKey::F23 => rdev::Key::Unknown(134),
            PTTKey::F24 => rdev::Key::Unknown(135),
            PTTKey::F2 => rdev::Key::F2,
            PTTKey::F3 => rdev::Key::F3,
            PTTKey::F4 => rdev::Key::F4,
            PTTKey::F5 => rdev::Key::F5,
            PTTKey::F6 => rdev::Key::F6,
            PTTKey::F7 => rdev::Key::F7,
            PTTKey::F8 => rdev::Key::F8,
            PTTKey::F9 => rdev::Key::F9,
            PTTKey::Home => rdev::Key::Home,
            PTTKey::LeftArrow => rdev::Key::LeftArrow,
            PTTKey::MetaLeft => rdev::Key::MetaLeft,
            PTTKey::MetaRight => rdev::Key::MetaRight,
            PTTKey::PageDown => rdev::Key::PageDown,
            PTTKey::PageUp => rdev::Key::PageUp,
            PTTKey::Return => rdev::Key::Return,
            PTTKey::RightArrow => rdev::Key::RightArrow,
            PTTKey::ShiftLeft => rdev::Key::ShiftLeft,
            PTTKey::ShiftRight => rdev::Key::ShiftRight,
            PTTKey::Space => rdev::Key::Space,
            PTTKey::Tab => rdev::Key::Tab,
            PTTKey::UpArrow => rdev::Key::UpArrow,
            PTTKey::PrintScreen => rdev::Key::PrintScreen,
            PTTKey::ScrollLock => rdev::Key::ScrollLock,
            PTTKey::Pause => rdev::Key::Pause,
            PTTKey::NumLock => rdev::Key::NumLock,
            PTTKey::BackQuote => rdev::Key::BackQuote,
            PTTKey::Num1 => rdev::Key::Num1,
            PTTKey::Num2 => rdev::Key::Num2,
            PTTKey::Num3 => rdev::Key::Num3,
            PTTKey::Num4 => rdev::Key::Num4,
            PTTKey::Num5 => rdev::Key::Num5,
            PTTKey::Num6 => rdev::Key::Num6,
            PTTKey::Num7 => rdev::Key::Num7,
            PTTKey::Num8 => rdev::Key::Num8,
            PTTKey::Num9 => rdev::Key::Num9,
            PTTKey::Num0 => rdev::Key::Num0,
            PTTKey::Minus => rdev::Key::Minus,
            PTTKey::Equal => rdev::Key::Equal,
            PTTKey::KeyQ => rdev::Key::KeyQ,
            PTTKey::KeyW => rdev::Key::KeyW,
            PTTKey::KeyE => rdev::Key::KeyE,
            PTTKey::KeyR => rdev::Key::KeyR,
            PTTKey::KeyT => rdev::Key::KeyT,
            PTTKey::KeyY => rdev::Key::KeyY,
            PTTKey::KeyU => rdev::Key::KeyU,
            PTTKey::KeyI => rdev::Key::KeyI,
            PTTKey::KeyO => rdev::Key::KeyO,
            PTTKey::KeyP => rdev::Key::KeyP,
            PTTKey::LeftBracket => rdev::Key::LeftBracket,
            PTTKey::RightBracket => rdev::Key::RightBracket,
            PTTKey::KeyA => rdev::Key::KeyA,
            PTTKey::KeyS => rdev::Key::KeyS,
            PTTKey::KeyD => rdev::Key::KeyD,
            PTTKey::KeyF => rdev::Key::KeyF,
            PTTKey::KeyG => rdev::Key::KeyG,
            PTTKey::KeyH => rdev::Key::KeyH,
            PTTKey::KeyJ => rdev::Key::KeyJ,
            PTTKey::KeyK => rdev::Key::KeyK,
            PTTKey::KeyL => rdev::Key::KeyL,
            PTTKey::SemiColon => rdev::Key::SemiColon,
            PTTKey::Quote => rdev::Key::Quote,
            PTTKey::BackSlash => rdev::Key::BackSlash,
            PTTKey::IntlBackslash => rdev::Key::IntlBackslash,
            PTTKey::KeyZ => rdev::Key::KeyZ,
            PTTKey::KeyX => rdev::Key::KeyX,
            PTTKey::KeyC => rdev::Key::KeyC,
            PTTKey::KeyV => rdev::Key::KeyV,
            PTTKey::KeyB => rdev::Key::KeyB,
            PTTKey::KeyN => rdev::Key::KeyN,
            PTTKey::KeyM => rdev::Key::KeyM,
            PTTKey::Comma => rdev::Key::Comma,
            PTTKey::Dot => rdev::Key::Dot,
            PTTKey::Slash => rdev::Key::Slash,
            PTTKey::Insert => rdev::Key::Insert,
            PTTKey::KpReturn => rdev::Key::KpReturn,
            PTTKey::KpMinus => rdev::Key::KpMinus,
            PTTKey::KpPlus => rdev::Key::KpPlus,
            PTTKey::KpMultiply => rdev::Key::KpMultiply,
            PTTKey::KpDivide => rdev::Key::KpDivide,
            PTTKey::Kp0 => rdev::Key::Kp0,
            PTTKey::Kp1 => rdev::Key::Kp1,
            PTTKey::Kp2 => rdev::Key::Kp2,
            PTTKey::Kp3 => rdev::Key::Kp3,
            PTTKey::Kp4 => rdev::Key::Kp4,
            PTTKey::Kp5 => rdev::Key::Kp5,
            PTTKey::Kp6 => rdev::Key::Kp6,
            PTTKey::Kp7 => rdev::Key::Kp7,
            PTTKey::Kp8 => rdev::Key::Kp8,
            PTTKey::Kp9 => rdev::Key::Kp9,
            PTTKey::KpDelete => rdev::Key::KpDelete,
            PTTKey::Function => rdev::Key::Function,
            PTTKey::Unknown(code) => rdev::Key::Unknown(code),
        }
    }
}
