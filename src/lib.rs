//! Macro Helldivers 2 — núcleo do app nativo.
//!
//! O crate expõe uma biblioteca além do binário para que os utilitários de
//! bancada (`src/bin/`) possam usar os mesmos módulos do app sem duplicar código.

pub mod bench;
pub mod data;
pub mod engine;
pub mod focus;
pub mod gfx;
pub mod hooks;
pub mod i18n;
pub mod keys;
pub mod settings;
pub mod shared;
pub mod ui;
pub mod util;
