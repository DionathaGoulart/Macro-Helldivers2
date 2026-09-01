//! Interface: tema, toolkit, widgets e a janela principal.
//!
//! Tudo o que decide layout, cor e reação a clique é lógica pura e roda no host;
//! só a janela em si (`window`) precisa do Windows.

pub mod macro_tab;
pub mod theme;
pub mod toolkit;
pub mod widgets;
pub mod window;
