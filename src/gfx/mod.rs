//! Camada gráfica: Direct2D, DirectWrite e os ícones do jogo.
//!
//! A decodificação de imagem é do host e roda em qualquer sistema; o resto só
//! existe no Windows, onde o app de fato roda.

pub mod images;

#[cfg(windows)]
pub mod d2d;
#[cfg(windows)]
pub mod text;
