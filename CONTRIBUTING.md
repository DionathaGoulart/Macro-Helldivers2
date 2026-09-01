# Contribuindo

A partir da v2.0.0 o app é um binário nativo em Rust + Win32 puro. **Ele só roda no
Windows.** Em macOS/Linux dá para compilar, checar e rodar os testes dos módulos de
lógica pura — todo código de janela, hook e input vive atrás de `#[cfg(windows)]`.

O app Electron/React da v1 está preservado em `legacy/` como referência de
comportamento durante a reescrita, e é removido quando a paridade for validada.

## Setup

```bash
# toolchain (stable; rust-toolchain.toml pina canal, componentes e alvo)
rustup target add x86_64-pc-windows-msvc

# cross-compilar do macOS/Linux sem Visual Studio
cargo install cargo-xwin
```

## Build

```bash
# Windows
cargo build --release --target x86_64-pc-windows-msvc

# macOS/Linux (cross)
cargo xwin build --release --target x86_64-pc-windows-msvc
```

## Gates de qualidade

Rodar antes de todo commit. **Sempre com `--target x86_64-pc-windows-msvc`** — é o
único alvo em que o código win32 é de fato compilado; sem isso os stubs escondem erros.

```bash
cargo fmt --all
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo check --target x86_64-pc-windows-msvc
cargo test            # módulos de lógica pura, no host
```

Teste funcional é sempre em máquina ou VM Windows, com o jogo aberto.

## Pipeline de dados (dev-only)

`assets/data/*.json` e `assets/icons/**` são gerados a partir da wiki por scripts Node
em `scripts/`. Não fazem parte do binário e não rodam em build normal:

```bash
cd scripts
npm run scrape            # equipment.json + imagens
npm run sync-stratagems   # stratagems.json + ícones
npm run stats-map         # statsMap.json (slugs do helldive.live)
npm run optimize-images   # PNG → WebP + reescrita das referências
```

## Commits

Conventional Commits, em inglês, pequenos e atômicos. A mensagem diz o QUE e o
PORQUÊ; corpo quando o motivo não for óbvio pelo diff.

## Regras invioláveis

- Nunca tocar no processo do jogo: sem injeção, sem leitura de memória, sem driver.
  Só APIs userland documentadas.
- Código win32 em módulos `#[cfg(windows)]`, com stub `#[cfg(not(windows))]` no-op
  suficiente para `cargo check`/`cargo test` rodarem no host.
