# Contribuindo

A partir da v2.0.0 o app é um binário nativo em Rust + Win32 puro. **Ele só roda no
Windows.** Em macOS/Linux dá para compilar, checar e rodar os testes dos módulos de
lógica pura — todo código de janela, hook e input vive atrás de `#[cfg(windows)]`.

O app Electron/React da v1 serviu de referência de comportamento durante a reescrita
e foi removido depois da validação de paridade. Ele segue acessível na tag
[`v1.0.0`](https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v1.0.0).

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

O build de debug abre com console, que é onde `RUST_LOG=debug cargo run` mostra os
logs. O de release não tem console e grava o log em `app.log`, na pasta
`%APPDATA%\Macro Helldivers 2`.

## Gates de qualidade

Rodar antes de todo commit. **Sempre com `--target x86_64-pc-windows-msvc`** — é o
único alvo em que o código win32 é de fato compilado; sem isso os stubs escondem erros.

```bash
cargo fmt --all
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo check --target x86_64-pc-windows-msvc
cargo test            # módulos de lógica pura, no host
```

A CI (`.github/workflows/ci.yml`) roda os mesmos gates em `windows-latest`, mais os
testes no alvo MSVC e um build de release, a cada push e PR.

Teste funcional é sempre em máquina ou VM Windows, com o jogo aberto. Para o motor de
macro há duas bancadas:

```bash
# desvio de timing (meta: p99 < 1 ms). Digita de verdade: rode com um editor de
# texto em foco, nunca com o jogo aberto.
cargo run --release --bin timing_bench

# mil chamadas dentro do jogo (meta: 0 falhas, a 60 e a 30 fps). O protocolo
# in-game está no topo de src/bin/soak.rs.
cargo run --release --bin soak -- -n 1000 --speed turbo
```

## Interface

O visual segue o [`styleguide.md`](styleguide.md) (skin `retro`, neobrutal): ele é a
fonte de verdade de tokens, componentes e dos dois temas (`rose` e `crimson`). Os
tokens vivem em `src/ui/theme.rs`; mudança de visual começa no styleguide e depois
desce para o código. O toolkit de widgets não desenha cantos arredondados, gradientes
nem brilhos — não aceita raio de propósito.

## Pipeline de dados (dev-only)

`assets/data/*.json` e `assets/icons/**` são gerados a partir da wiki por scripts Node
em `scripts/`. Não fazem parte do binário e não rodam em build normal:

```bash
cd scripts
npm install               # sharp + resvg; nenhum binário externo é necessário

npm run scrape            # equipment.json + imagens (--refresh ignora o cache)
npm run optimize-images   # PNG → WebP + reescrita das referências
npm run sync-stratagems   # stratagems.json + ícones
npm run stats-map         # statsMap.json (slugs do helldive.live)
```

A ordem importa: `optimize-images` converte os `.png` que o `scrape` acabou de baixar
e reescreve as referências, e `stats-map` valida se os nomes novos ainda casam com os
slugs do helldive.live. O `sync-stratagems` casa por código de entrada e preserva os
IDs — nunca renumere estratagemas à mão, ou slots e builds salvas dos usuários passam
a apontar para outro item.

## Commits

Conventional Commits, em inglês, pequenos e atômicos. A mensagem diz o QUE e o
PORQUÊ; corpo quando o motivo não for óbvio pelo diff.

## Release

A tag é a única fonte da versão: o `release.yml` reescreve a versão do `Cargo.toml`
com a da tag dentro do runner, e o updater compara essa versão com o `tag_name` do
GitHub.

1. Mova as entradas para uma seção `## [X.Y.Z] - AAAA-MM-DD` no
   [`CHANGELOG.md`](CHANGELOG.md) e adicione o link de comparação no fim do arquivo.
   **Essa seção vira o texto do release.**
2. Atualize a versão no `Cargo.toml` e no título do `README.md`.
3. Crie e publique a tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.

A CI roda os testes, compila, gera o instalador NSIS
(`Macro-Helldivers-2-Setup-X.Y.Z.exe`) e o `.sha256` e publica o release. Tags com
sufixo (`vX.Y.Z-beta.N`) saem como pré-release; sem seção no changelog, as notas são
as geradas pelo GitHub.

## Regras invioláveis

- Nunca tocar no processo do jogo: sem injeção, sem leitura de memória, sem driver.
  Só APIs userland documentadas.
- Código win32 em módulos `#[cfg(windows)]`, com stub `#[cfg(not(windows))]` no-op
  suficiente para `cargo check`/`cargo test` rodarem no host.
