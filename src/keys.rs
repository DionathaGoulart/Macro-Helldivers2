//! Tabelas canônicas de teclas.
//!
//! Os nomes são exatamente os que a v1 gravava em `settings.json` — trocá-los
//! invalidaria os atalhos de quem migra. Deles saem duas coisas: o virtual-key
//! que o hook de teclado compara, e o scancode que o `SendInput` do engine
//! manda (o jogo lê scancode; VK não chega nele).

use crate::data::Dir;

/// Virtual-key do Win32.
pub type Vk = u16;

/// Scancode do set 1, do jeito que o `SendInput` precisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Scan {
    pub code: u16,
    /// Teclas do bloco estendido exigem `KEYEVENTF_EXTENDEDKEY`.
    pub extended: bool,
}

impl Scan {
    const fn plain(code: u16) -> Scan {
        Scan {
            code,
            extended: false,
        }
    }

    const fn ext(code: u16) -> Scan {
        Scan {
            code,
            extended: true,
        }
    }
}

/// Esc não é atribuível: cancela a captura de atalho.
pub const VK_ESCAPE: Vk = 0x1B;

/// Virtual-keys de modificador puro, que a captura ignora e o hook nunca dispara.
const MODIFIER_VKS: [Vk; 11] = [
    0x10, // VK_SHIFT
    0x11, // VK_CONTROL
    0x12, // VK_MENU
    0x5B, // VK_LWIN
    0x5C, // VK_RWIN
    0xA0, // VK_LSHIFT
    0xA1, // VK_RSHIFT
    0xA2, // VK_LCONTROL
    0xA3, // VK_RCONTROL
    0xA4, // VK_LMENU
    0xA5, // VK_RMENU
];

/// Nome canônico → virtual-key. Nomes e VKs são únicos, então a busca reversa
/// é determinística.
const KEYS: &[(&str, Vk)] = &[
    ("F1", 0x70),
    ("F2", 0x71),
    ("F3", 0x72),
    ("F4", 0x73),
    ("F5", 0x74),
    ("F6", 0x75),
    ("F7", 0x76),
    ("F8", 0x77),
    ("F9", 0x78),
    ("F10", 0x79),
    ("F11", 0x7A),
    ("F12", 0x7B),
    ("0", 0x30),
    ("1", 0x31),
    ("2", 0x32),
    ("3", 0x33),
    ("4", 0x34),
    ("5", 0x35),
    ("6", 0x36),
    ("7", 0x37),
    ("8", 0x38),
    ("9", 0x39),
    ("A", 0x41),
    ("B", 0x42),
    ("C", 0x43),
    ("D", 0x44),
    ("E", 0x45),
    ("F", 0x46),
    ("G", 0x47),
    ("H", 0x48),
    ("I", 0x49),
    ("J", 0x4A),
    ("K", 0x4B),
    ("L", 0x4C),
    ("M", 0x4D),
    ("N", 0x4E),
    ("O", 0x4F),
    ("P", 0x50),
    ("Q", 0x51),
    ("R", 0x52),
    ("S", 0x53),
    ("T", 0x54),
    ("U", 0x55),
    ("V", 0x56),
    ("W", 0x57),
    ("X", 0x58),
    ("Y", 0x59),
    ("Z", 0x5A),
    ("Numpad0", 0x60),
    ("Numpad1", 0x61),
    ("Numpad2", 0x62),
    ("Numpad3", 0x63),
    ("Numpad4", 0x64),
    ("Numpad5", 0x65),
    ("Numpad6", 0x66),
    ("Numpad7", 0x67),
    ("Numpad8", 0x68),
    ("Numpad9", 0x69),
    ("Up", 0x26),
    ("Down", 0x28),
    ("Left", 0x25),
    ("Right", 0x27),
    ("Space", 0x20),
    ("Tab", 0x09),
    ("Backspace", 0x08),
    ("Delete", 0x2E),
    ("Home", 0x24),
    ("End", 0x23),
    ("Insert", 0x2D),
    ("PageUp", 0x21),
    ("PageDown", 0x22),
    ("LeftShift", 0xA0),
    ("LeftControl", 0xA2),
    ("LeftAlt", 0xA4),
    ("Equal", 0xBB), // VK_OEM_PLUS
    ("Minus", 0xBD), // VK_OEM_MINUS
];

/// Virtual-key de um nome canônico.
pub fn vk_from_name(name: &str) -> Option<Vk> {
    KEYS.iter().find(|(key, _)| *key == name).map(|(_, vk)| *vk)
}

/// Nome canônico de um virtual-key.
pub fn name_from_vk(vk: Vk) -> Option<&'static str> {
    KEYS.iter()
        .find(|(_, key)| *key == vk)
        .map(|(name, _)| *name)
}

/// Modificadores puros não viram atalho: a captura os ignora, como na v1.
pub fn is_modifier_vk(vk: Vk) -> bool {
    MODIFIER_VKS.contains(&vk)
}

/// Teclas de estratagema aceitas pelo jogo, na ordem em que a UI as mostra.
pub const MODIFIER_KEYS: [&str; 4] = ["LeftControl", "LeftAlt", "Equal", "Minus"];

/// Scancode do modificador in-game. Nome inválido cai no padrão do jogo (Ctrl).
pub fn modifier_scan(name: &str) -> Scan {
    match name {
        "LeftAlt" => Scan::plain(0x38),
        "Equal" => Scan::plain(0x0D),
        "Minus" => Scan::plain(0x0C),
        // "LeftControl" e qualquer coisa que não reconhecemos.
        _ => Scan::plain(0x1D),
    }
}

/// W A S D, indexado por [`Dir`].
pub const WASD: [Scan; 4] = [
    Scan::plain(0x11), // W
    Scan::plain(0x1F), // S
    Scan::plain(0x1E), // A
    Scan::plain(0x20), // D
];

/// Setas do bloco estendido, indexadas por [`Dir`].
pub const ARROWS: [Scan; 4] = [
    Scan::ext(0x48), // ↑
    Scan::ext(0x50), // ↓
    Scan::ext(0x4B), // ←
    Scan::ext(0x4D), // →
];

/// Scancode de um passo do codex, conforme o modo de setas do usuário.
pub fn direction_scan(dir: Dir, use_arrows: bool) -> Scan {
    if use_arrows {
        ARROWS[dir.index()]
    } else {
        WASD[dir.index()]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_and_vks_are_unique() {
        let names: HashSet<_> = KEYS.iter().map(|(name, _)| *name).collect();
        let vks: HashSet<_> = KEYS.iter().map(|(_, vk)| *vk).collect();
        assert_eq!(names.len(), KEYS.len());
        assert_eq!(vks.len(), KEYS.len());
    }

    #[test]
    fn every_name_round_trips_through_its_vk() {
        for (name, vk) in KEYS {
            assert_eq!(vk_from_name(name), Some(*vk));
            assert_eq!(name_from_vk(*vk), Some(*name));
        }
    }

    #[test]
    fn v1_default_shortcuts_resolve() {
        for name in ["F1", "F2", "F3", "F4"] {
            assert!(vk_from_name(name).is_some(), "{name}");
        }
        assert_eq!(vk_from_name("LeftControl"), Some(0xA2));
        assert_eq!(vk_from_name("Nope"), None);
    }

    #[test]
    fn pure_modifiers_are_not_bindable_shortcuts() {
        assert!(is_modifier_vk(vk_from_name("LeftControl").unwrap()));
        assert!(is_modifier_vk(vk_from_name("LeftShift").unwrap()));
        assert!(is_modifier_vk(vk_from_name("LeftAlt").unwrap()));
        assert!(!is_modifier_vk(vk_from_name("F1").unwrap()));
        assert!(!is_modifier_vk(VK_ESCAPE));
    }

    #[test]
    fn modifier_scancodes_match_the_reference_table() {
        assert_eq!(modifier_scan("LeftControl"), Scan::plain(0x1D));
        assert_eq!(modifier_scan("LeftAlt"), Scan::plain(0x38));
        assert_eq!(modifier_scan("Equal"), Scan::plain(0x0D));
        assert_eq!(modifier_scan("Minus"), Scan::plain(0x0C));
        // Um settings.json corrompido não pode deixar o macro sem modificador.
        assert_eq!(modifier_scan("Bogus"), modifier_scan("LeftControl"));
        for name in MODIFIER_KEYS {
            assert!(vk_from_name(name).is_some(), "{name}");
        }
    }

    #[test]
    fn direction_scancodes_match_the_reference_table() {
        assert_eq!(direction_scan(Dir::Up, false), Scan::plain(0x11)); // W
        assert_eq!(direction_scan(Dir::Left, false), Scan::plain(0x1E)); // A
        assert_eq!(direction_scan(Dir::Down, false), Scan::plain(0x1F)); // S
        assert_eq!(direction_scan(Dir::Right, false), Scan::plain(0x20)); // D

        assert_eq!(direction_scan(Dir::Up, true), Scan::ext(0x48));
        assert_eq!(direction_scan(Dir::Down, true), Scan::ext(0x50));
        assert_eq!(direction_scan(Dir::Left, true), Scan::ext(0x4B));
        assert_eq!(direction_scan(Dir::Right, true), Scan::ext(0x4D));

        // As setas são todas estendidas; WASD, nenhuma.
        assert!(ARROWS.iter().all(|s| s.extended));
        assert!(WASD.iter().all(|s| !s.extended));
    }
}
