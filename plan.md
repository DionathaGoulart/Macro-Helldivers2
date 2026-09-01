# Plano de Reescrita — Macro Helldivers 2 v2.0.0 (Win32 puro + Rust)

**Branch:** `rewrite/rust-native` (baseada em `development`)

**Stack decidida (final):**

| # | Parte | Escolha |
|---|---|---|
| 1 | Shell | **Win32 cru** — `CreateWindowExW` + message loop próprio, 1 processo, zero framework de janela |
| 2 | Core | **Rust** (crate `windows` oficial = bindings zero-cost da API; sem GC, sem runtime) |
| 3 | Input | **`SendInput` com scancode** (`KEYEVENTF_SCANCODE`) |
| 4 | Timing | **`spin_sleep` + `timeBeginPeriod(1)`** |
| 5 | Hotkeys | **`WH_KEYBOARD_LL`** (low-level keyboard hook) |
| 6 | Foco | **`SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`** — event-driven, zero polling |
| 7 | Overlay | **Direct2D em layered windows nativas** (strip + painel) |
| 8 | UI | **Win32 cru + Direct2D/DirectWrite** — toolkit de widgets próprio, redraw só em mudança de estado |
| 9 | Updater | **NSIS + auto-update via GitHub Releases** (download manual, instalação confirmada) |

**Metas:** RAM total < 20MB · CPU idle ~0% · boot < 300ms · latência atalho→primeira tecla < 5ms · desvio de timing < 1ms · zero falha de estratagema em 1.000 execuções turbo · impacto de FPS imperceptível · instalador < 5MB.

**Mudanças deliberadas vs v1 (decididas pelo dono do projeto — NÃO são regressão):**

1. **"Modificador de sprint" removido** — o LL hook dispara com qualquer modificador seguro; o hack ficou obsoleto.
2. **Boot intro (animação CRT 4,5s) removida** — app abre direto na UI; boot instantâneo é feature.
3. **Painel do overlay enxuto** — só aba Macros (atribuir slots) + Builds salvas (aplicar). Configurações e criação/geração de builds: só na janela principal.
4. Campos de texto usam controle `EDIT` nativo skinnado (filho, estilizado escuro); todo o resto da UI é desenhado em D2D.

---

## COMO USAR ESTE PLANO (instruções para o agente implementador)

1. **Uma fase por sessão.** Leia a seção "Referência global" (§R) e a fase atual INTEIRA antes de escrever código. Não leia fases futuras.
2. **Consulte o legado como fonte de verdade de comportamento.** O código Electron/React vive em `legacy/` (a partir da Fase 0). Quando a fase mandar "portar lógica X de `legacy/src/...`", abra o arquivo e reproduza o comportamento exatamente — números, regras, ordem de operações.
3. **Gates de qualidade em toda fase**, antes de commitar:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
   cargo check --target x86_64-pc-windows-msvc
   cargo test   # módulos de lógica pura rodam no host (macOS)
   ```
4. **Protocolo de commit (obrigatório):**
   - Conventional Commits, em **inglês**: `feat(engine): ...`, `refactor: ...`, `chore: ...`, `fix(overlay): ...`.
   - Mensagens descrevem O QUE e POR QUÊ; corpo quando não for óbvio.
   - Autor = o `git user` já configurado no repo (DionathaGoulart). **PROIBIDO**: `Co-Authored-By`, trailer `Claude-Session`, qualquer menção a IA/Claude. Commits limpos, como escritos por um dev humano.
   - Commits pequenos e atômicos — cada fase lista os commits esperados; dividir mais é permitido, juntar não.
5. **Fim de fase:** commitar tudo, `git status` limpo, e encerrar a resposta com:
   > ✅ Fase N concluída e commitada. **Recomendo rodar `/clear` agora** e abrir a Fase N+1 numa sessão nova (peça: "implemente a Fase N+1 do plan.md").
6. **Política de plataforma:** TODO código win32 em módulos `#[cfg(windows)]` com stub `#[cfg(not(windows))]` (no-op/valor neutro) só o bastante pro `cargo check`/`cargo test` dos módulos puros rodarem no macOS de dev. **O app em si só roda no Windows** — build via `cargo xwin build --release --target x86_64-pc-windows-msvc` ou CI; teste funcional em máquina/VM Windows.
7. **Nunca** tocar no processo do jogo: sem injeção, sem leitura de memória, sem driver. Só APIs userland documentadas.

---

## §R — REFERÊNCIA GLOBAL (consultar em todas as fases)

### R1. Layout final do repositório

```
Cargo.toml                  # binário único "macro-helldivers2"
rust-toolchain.toml         # stable pinado
build.rs                    # winresource: icon.ico + manifest (requireAdministrator + perMonitorV2)
src/
├── main.rs                 # bootstrap: single-instance, threads, janela principal, message loop
├── shared.rs               # Arc<Shared>: estado entre threads + canais (R4)
├── settings.rs             # load/save/migração de settings.json (R6)
├── keys.rs                 # nome↔VK↔scancode (R7)
├── data.rs                 # structs + parse de stratagems/equipment/statsMap (lazy p/ equipment)
├── i18n.rs                 # traduções pt/en (port integral de legacy translations.js)
├── engine.rs               # ★ motor de macro: thread, SendInput, perfis, RAII guards
├── hooks.rs                # thread única: WH_KEYBOARD_LL + SetWinEventHook + pump
├── focus.rs                # classificação da janela em foco + máquina de estados
├── game_config.rs          # user_settings.config do HD2 (cache mtime)
├── builds.rs               # lógica PURA de geração (aleatória/meta/regras/locks) — testável no host
├── meta_stats.rs           # fetch helldive.live + cache 6h em disco
├── loadouts.rs             # slots.json + loadouts.json + backup import/export
├── updater.rs              # GitHub Releases: check/download/install
├── tray.rs                 # Shell_NotifyIconW cru + menu popup
├── gfx/
│   ├── d2d.rs              # factories D2D/DWrite/WIC-free, render targets, DIB helpers
│   ├── text.rs             # DirectWrite: coleção de fontes custom (Inter embutida), formats cacheados
│   └── images.rs           # decode WebP (crate image) → ID2D1Bitmap, cache por caminho
├── ui/
│   ├── theme.rs            # tokens de cor/raio/espaçamento (R5), escala DPI
│   ├── toolkit.rs          # ★ widget kit D2D: árvore, layout, hit-test, invalidação, scroll, timers
│   ├── widgets.rs          # button, toggle, tab_bar, hd_card, slot_square, arrow_row, dropdown, edit_host
│   ├── window.rs           # janela principal: class, WndProc, resize/DPI, bounds persistidos
│   ├── macro_tab.rs        # grade de estratagemas + slots
│   ├── build_tab.rs        # meta / aleatória / personalizada / salvas
│   ├── settings_tab.rs     # atalhos, modificadores, idioma, backup, updater UI
│   └── modal.rs            # modal genérico D2D (update pronto, confirmações)
├── overlay/
│   ├── mod.rs              # estados hidden/minimal/panel, bounds, comandos, toggle
│   ├── strip.rs            # ★ strip D2D layered (thread overlay)
│   └── panel.rs            # painel D2D layered interativo (mesma thread; usa ui/toolkit)
└── util.rs                 # paths, logging, single-instance, dialogs COM (IFileDialog)
assets/
├── data/                   # stratagems.json, equipment.json, statsMap.json
├── icons/                  # icon.png, tray.png, 3 webp de apoio, stratagems/ (91), equipment/ (470)
├── fonts/                  # Inter-Regular.ttf, Inter-Black.ttf (OFL)
└── icon.ico
installer/installer.nsi     # NSIS perMachine/admin
.github/workflows/          # ci.yml, release.yml
scripts/                    # pipeline da wiki (Node) — caminhos atualizados na Fase 0
legacy/                     # app Electron/React (referência; deletado na fase final)
```

### R2. Crates (versões estáveis mais recentes ao implementar)

```toml
[dependencies]
serde = { version = "*", features = ["derive"] }
serde_json = "*"
crossbeam-channel = "*"
spin_sleep = "*"
rand = "*"
ureq = { version = "*", features = ["json"] }
semver = "*"
image = { version = "*", default-features = false, features = ["webp", "png"] }
directories = "*"
log = "*"
env_logger = "*"
anyhow = "*"

[target.'cfg(windows)'.dependencies]
windows = { version = "*", features = [
  "Win32_Foundation", "Win32_UI_Input_KeyboardAndMouse", "Win32_UI_WindowsAndMessaging",
  "Win32_UI_Accessibility", "Win32_UI_Controls", "Win32_UI_Shell", "Win32_UI_HiDpi",
  "Win32_Graphics_Direct2D", "Win32_Graphics_Direct2D_Common", "Win32_Graphics_DirectWrite",
  "Win32_Graphics_Gdi", "Win32_Graphics_Dwm", "Win32_System_Com", "Win32_System_Threading",
  "Win32_System_LibraryLoader", "Win32_Media", "Win32_Storage_FileSystem",
] }

[build-dependencies]
winresource = "*"
```

**Sem** framework de UI, sem winit, sem tokio, sem tray-icon, sem rfd — janela, tray, dialogs e render são win32/COM crus. Crates restantes são lógica (JSON, WebP, HTTP, timing) e não tocam o runtime de UI.

### R3. Threads e responsabilidades

| Thread | Vive | Faz |
|---|---|---|
| **main** | sempre | message loop da janela principal (UI D2D) + tray + modais |
| **hooks** | sempre | pump com `WH_KEYBOARD_LL` + `SetWinEventHook`; callbacks só comparam e postam em canal — NUNCA bloqueiam (deadline do SO ~300ms) |
| **engine** | sempre | executa sequências de macro; `THREAD_PRIORITY_HIGHEST` |
| **overlay** | se `enableOverlay` | strip + painel (duas layered windows D2D) + pump + timer z-order |
| **workers** | efêmeras | meta stats, updater check/download, escrita de arquivos |

Comunicação inter-thread: canais crossbeam + `PostMessageW(WM_APP+n)` para acordar o pump alvo (cada thread com janela drena seu receiver ao receber o WM_APP correspondente).

### R4. Estado compartilhado e canais (`shared.rs`)

```rust
pub struct Shared {
    pub settings: RwLock<Settings>,
    pub slots: RwLock<[Option<u32>; 4]>,        // ids de estratagema
    pub game_focused: AtomicBool,
    pub recording: AtomicBool,                   // captura de tecla ativa → hook não engole/dispara
    pub macro_running: AtomicBool,
    pub overlay_state: RwLock<OverlayState>,     // Hidden | Minimal | Panel
    pub engine_tx: Sender<EngineCmd>,
    pub overlay_tx: Sender<OverlayCmd>,
    pub ui_tx: Sender<UiEvent>,                  // main window drena via WM_APP_UI_EVENT
    pub main_hwnd: AtomicIsize,                  // pra PostMessageW de qualquer thread
    pub overlay_hwnd: AtomicIsize,
}

pub enum EngineCmd { Run { codex: Vec<Dir>, modifier: Vk, use_arrows: bool, speed: Speed, slot: usize, support: bool } }
pub enum UiEvent  { GameFocus(bool), MacroTriggered{slot:usize,support:bool}, MacroBlocked{slot:usize,support:bool},
                    MacroStatus{slot:usize,support:bool,running:bool}, FullscreenWarning(bool),
                    OverlayState(OverlayState), UpdateStatus(UpdateStatus), MetaStats(MetaResult) }
pub enum OverlayCmd { SetState(OverlayState), Toggle, Slots([Option<u32>;4]), LoadoutsChanged,
                      Flash{slot:usize,kind:FlashKind}, Reassert, FullscreenWarning(bool) }
```

Caminho quente (hotkey→tecla) NÃO passa pela UI: hooks → `engine_tx` → SendInput. UI é notificada depois.

### R5. Tema (tokens exatos do legado — `ui/theme.rs`)

| Token | Valor | Uso |
|---|---|---|
| `BG_DEEP` | `#020617` | fundo da janela |
| `CARD_BG` | `#0B1120` (slate-900/40 pré-composto sobre BG) | hd_card |
| `BORDER` | `#1E293B` (slate-800) | bordas padrão |
| `TEXT` | `#E2E8F0` (slate-200) | texto |
| `TEXT_DIM` | `#64748B` (slate-500) | labels |
| `YELLOW` | `#FBBF24` | acento principal/ativo |
| `CYAN` | `#22D3EE` | acento secundário/Supply |
| `RED` | `#EF4444` | Offensive/erros/bloqueio |
| `GREEN` | `#22C55E` | Defensive/sucesso |

Raios: cards 16px, botões 12px, slots 12px. Fonte: **Inter** (Regular 400 + Black 900) via coleção DirectWrite custom carregada de `assets/fonts/` (sem instalar no sistema). **Fidelidade: réplica fiel do tema** — rounded rects, gradientes lineares (headers dos cards de estratagema), glow aproximado com stroke externo semitransparente 2–3px na cor do acento (sem gaussian blur). Todos os valores em DIP × scale do monitor (R11-DPI).

### R6. Settings — schema e migração

Arquivo: `%APPDATA%\Macro Helldivers 2\settings.json` (`directories::ProjectDirs("com","macro","Macro Helldivers 2")`, `config_dir`).

```json
{
  "shortcuts": ["F1", "F2", "F3", "F4"],
  "supportShortcuts": [null, null, null],
  "modifierKey": "LeftControl",
  "useArrows": false,
  "macroSpeed": "normal",
  "language": "pt",
  "enableOverlay": true,
  "alwaysShowSlots": false,
  "buildMatchSet": true,
  "buildBalanced": false,
  "buildMaxOneSentry": false
}
```

serde `rename_all = "camelCase"`, `#[serde(default)]` por campo com os defaults do legado, campos desconhecidos ignorados (`sprintModifier` da v1 morre sozinho). Migração v1: se o arquivo novo não existe, tentar `%APPDATA%\helldivers-macro\Helldivers Macro\settings.json` e salvar no caminho novo. Slots/loadouts da v1 (localStorage/LevelDB) NÃO migram — caminho suportado: backup JSON (R9).

### R7. Teclas — tabelas canônicas (`keys.rs`)

Nomes string idênticos aos da v1: `"F1".."F12"`, `"A".."Z"`, `"0".."9"`, `"Numpad0".."Numpad9"`, `"Up"/"Down"/"Left"/"Right"`, `"Space"`, `"Tab"`, `"Backspace"`, `"Delete"`, `"Home"`, `"End"`, `"Insert"`, `"PageUp"`, `"PageDown"`, `"LeftControl"`, `"LeftAlt"`, `"Equal"`, `"Minus"`.

Scancodes (set 1) usados pelo engine:

| Tecla | Scancode | Flags |
|---|---|---|
| W | `0x11` | — |
| A | `0x1E` | — |
| S | `0x1F` | — |
| D | `0x20` | — |
| ↑ | `0x48` | `KEYEVENTF_EXTENDEDKEY` |
| ↓ | `0x50` | `KEYEVENTF_EXTENDEDKEY` |
| ← | `0x4B` | `KEYEVENTF_EXTENDEDKEY` |
| → | `0x4D` | `KEYEVENTF_EXTENDEDKEY` |
| LCtrl | `0x1D` | — |
| LAlt | `0x38` | — |
| `=` | `0x0D` | — |
| `-` | `0x0C` | — |

Envio: `SendInput` com `KEYEVENTF_SCANCODE` (wVk=0); release adiciona `KEYEVENTF_KEYUP`. Resto do mapa nome→VK (hotkeys) via constantes `VK_*` + `MapVirtualKeyW` quando precisar de scancode genérico.

### R8. Perfis de velocidade (números idênticos ao legado — `engine.rs`)

```
normal: hold 34ms, gap 20ms, lead 100ms, tail 50ms   (~1 frame @30fps)
fast:   hold 24ms, gap 12ms, lead 70ms,  tail 40ms   (~1,5 frame @60fps)
turbo:  hold 20ms, gap 6ms,  lead 50ms,  tail 30ms   (~1,2 frame @60fps)
MIN_HOLD_MS = 20  (piso inegociável: o jogo lê teclado 1×/frame)
jitter = valor + rand(-5..=+5)ms, com clamp no piso (hold nunca < 20)
```

Sequência: press modificador → lead → por direção {press → hold → release → gap} → tail → release modificador. Aborto (perda de foco): parar imediatamente, guard solta tudo.

### R9. Persistência e formatos

| Arquivo (em `config_dir`) | Conteúdo |
|---|---|
| `settings.json` | R6 |
| `slots.json` | `[id\|null; 4]` — re-resolvidos por id contra stratagems.json no load; conflitos de exclusividade herdados limpos (portar de `legacy/src/renderer/App.jsx` ~82–98) |
| `loadouts.json` | `[{ id, name, slotIds: [id|null;4], equip?: {slot: itemId} }]` |
| `meta-cache.json` | `{ "faction|difficulty": { at: epoch_ms, data } }`, TTL 6h |
| `window-bounds.json` | bounds da janela principal, validados contra monitores (portar de `legacy/src/main/index.js` ~436–469) |

Backup (export/import via `IFileDialog` COM): **exatamente** o formato v1 → `{ "app": "macro-helldivers2", "exportedAt": ISO, "settings": {...}, "loadouts": [...], "slotIds": [...] }`. Import valida `app`, merge de settings, re-resolve slotIds, limpa conflitos (portar de `legacy/src/renderer/App.jsx` ~199–241).

### R10. Classificação de foco (portar de `legacy/src/main/index.js` ~262–267)

Título uppercase da janela em foreground:
- contém `HD2_OVERLAY` → overlay (nosso — strip E painel usam esse título de janela)
- contém `MACRO` E `HELLDIVERS` → app (janela principal chama-se `Macro Helldivers 2`)
- contém `HELLDIVERS` (e não é o app) → **jogo**
- focado = jogo ∨ app ∨ overlay

Transições (máquina do legado): ganhou foco → hotkeys ativos, overlay `minimal` se `alwaysShowSlots`, aviso fullscreen; perdeu → hotkeys de slot inativos, overlay `hidden`, check de update adiado dispara.

### R11. Overlay — geometria, DPI e regras

Janelas do overlay SEMPRE visíveis, NUNCA show/hide (regra v1: mostrar janela rouba foco); mudam bounds:
- `hidden` → strip 1×1 no canto do monitor primário; painel 1×1 também (ou fora da tela)
- `minimal` → strip 340×130 DIP, bottom-center
- `panel` → painel 840×660 DIP, centralizado (strip vai pra 1×1)

DIP→px: `GetDpiForMonitor`/per-monitor-v2; reagir a `WM_DPICHANGED`/`WM_DISPLAYCHANGE`. Toggle Ctrl+H no LL hook (VK `H` + `GetAsyncKeyState(VK_CONTROL)`), só com jogo focado e `enableOverlay`. Estilos: `WS_POPUP` + `WS_EX_LAYERED | WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW | WS_EX_TOPMOST`; strip sempre com `WS_EX_TRANSPARENT` (click-through); painel SEM `WS_EX_TRANSPARENT` quando em estado panel (recebe mouse sem ativar — NOACTIVATE garante). Z-order: `SetWindowPos(HWND_TOPMOST, SWP_NOACTIVATE)` no evento de foco do jogo + timer 5s enquanto visível. Aviso fullscreen exclusivo: `game_config.rs` lê `%APPDATA%\Arrowhead\Helldivers2\user_settings.config`, cache mtime, regex `^\s*fullscreen\s*=\s*true` sem `^\s*borderless_fullscreen\s*=\s*true` (portar de `legacy/src/main/index.js` ~31–44).

### R12. Apoios fixos (`data.rs`, portar de `legacy/src/shared/constants.js`)

```
Reinforce:   UP DOWN RIGHT LEFT UP
Resupply:    DOWN DOWN UP RIGHT
Eagle Rearm: UP UP LEFT UP RIGHT
```

### R13. Meta stats (portar de `legacy/src/main/index.js` ~682–698 e `legacy/src/renderer/components/BuildTab.jsx`)

- `https://utm7j5pjvi.us-east-1.awsapprunner.com/items_stats?faction={f}&patch_id=12&difficulty={d}&mission=All&modifier=ALL&type={t}`, `t ∈ {strategem, weapons, armor}`; timeout 10s; erro degrada com mensagem.
- `PATCH_ID = 12` constante comentada ("atualizar quando o site adicionar patch novo").
- Facções terminid/automaton/illuminate; dificuldades 0 (All), 7–10.
- Slug→item via `assets/data/statsMap.json`.

### R14. Modelo de render da UI (vale pra janela principal E painel)

- **Zero loop de render.** Pintura só em `WM_PAINT`; invalidação (`InvalidateRect`) SOMENTE em: input do usuário, `UiEvent` recebido, timer de animação ativo (flash/hover-fade) — timers (`SetTimer` 16ms) existem apenas enquanto a animação corre e são mortos ao fim. App parado = 0% CPU/GPU.
- Janela principal: `ID2D1HwndRenderTarget` (resize em `WM_SIZE`). Strip/painel: DIB 32bpp premultiplicado + `ID2D1DCRenderTarget` + `UpdateLayeredWindow` (per-pixel alpha sobre o jogo).
- Toolkit (`ui/toolkit.rs`): árvore de widgets com rects calculados por layout simples (colunas/linhas/grid com gap e padding — nada genérico demais: o que as telas precisam), hit-test por retângulo no mouse move/click, estado de hover/pressed/focus por id, scroll container com wheel + barra desenhada (estilo `scrollbar-hd`: 5px, thumb slate). Widgets desenham via trait sobre `ID2D1RenderTarget` (funciona nos dois tipos de target).
- Texto: DirectWrite, `IDWriteTextFormat` cacheados por (família, peso, tamanho); uppercase/letter-spacing do tema aplicados na construção da string/format.
- `EDIT` nativo como filho para inputs de texto (busca, nome da build): fonte via `WM_SETFONT`, cores via `WM_CTLCOLOREDIT` (fundo escuro, texto claro), borda desenhada pelo pai em D2D; notificações `EN_CHANGE` disparam re-filtro/invalidate.
- Dropdown (selects de equipamento): popup `WS_POPUP` D2D com lista rolável, fecha em kill-focus/clique-fora/Esc.

---

## FASES

---

### FASE 0 — Reestruturação do repositório + scaffold Rust

**Objetivo:** repo pronto pra Rust, Electron preservado em `legacy/`, assets no lugar definitivo, pipeline da wiki com caminhos novos.

**Tarefas:**

1. Mover pra `legacy/`: `src/`, `index.html`, `vite.config.js`, `eslint.config.js`, `package.json`, `package-lock.json`.
2. Assets:
   - `git mv legacy/src/renderer/data assets/data`
   - `git mv public assets/icons` (mantém `stratagems/`, `equipment/`, `icon.png`, `tray.png`, 3 webp de apoio na raiz de `icons/`).
   - Caminhos `imagem` nos JSONs são relativos (`stratagems/...`, `equipment/...`) — o app resolve contra `assets/icons/`; nada a editar nos JSONs.
3. `scripts/*.mjs`: atualizar constantes `OUT_JSON`/`IMG_DIR`/etc. pros caminhos novos. Criar `scripts/package.json` mínimo com os 4 npm scripts (scrape, sync-stratagems, stats-map, optimize-images).
4. Scaffold Rust na raiz: `Cargo.toml` (nome `macro-helldivers2`; perfil release: `lto = "thin"`, `strip = true`, `opt-level = 3`, `panic = "abort"` SÓ em release — dev mantém unwind pros guards/testes), `rust-toolchain.toml` (stable), `rustfmt.toml`, `src/main.rs` placeholder, `.gitignore` += `target/`.
5. `CONTRIBUTING.md` curto: `rustup target add x86_64-pc-windows-msvc`, `cargo install cargo-xwin`, build: `cargo xwin build --release --target x86_64-pc-windows-msvc`; gates sempre com `--target x86_64-pc-windows-msvc`; app só roda em Windows, testes de lógica rodam no host.
6. CI `.github/workflows/ci.yml` (`windows-latest`): fmt --check, clippy -D warnings, check, test.
7. Gerar `assets/icon.ico`: `magick assets/icons/icon.png -define icon:auto-resize=256,64,48,32,16 assets/icon.ico` e commitar.
8. Baixar Inter (Regular + Black, licença OFL) → `assets/fonts/`, commitar.

**Critérios de aceite:** `cargo run` roda o placeholder; constantes dos scripts conferidas; CI verde; `legacy/` navegável.

**Commits:**
1. `refactor: move electron app to legacy/ for reference during rewrite`
2. `refactor: relocate data and image assets to assets/`
3. `chore: point data pipeline scripts at new asset layout`
4. `chore: scaffold rust binary crate with ci workflow`
5. `chore: add app icon ico and inter font assets`

> ✅ Commitar, working tree limpa, **recomendar `/clear`** antes da Fase 1.

---

### FASE 1 — Fundação lógica: settings, dados, i18n, paths

**Objetivo:** todos os módulos SEM janela prontos e testados no host. (A janela vem na Fase 4 — antes dela o motor já estará pronto e benchmarkável via bins de console.)

**Tarefas:**

1. `util.rs`: paths (`directories`, R6), `init_logging()`, `asset_path()` — debug: `CARGO_MANIFEST_DIR/assets`; release: diretório do exe + `assets`.
2. `settings.rs`: struct `Settings` (R6), `load()` com migração v1, `save()` atômico (temp + rename). Testes: parse de settings.json v1 com `sprintModifier` → ignorado; round-trip.
3. `data.rs`: `Stratagem { id, nome, imagem, tag: Vec<String>, codex: Vec<Dir> }`, `Dir` (serde UPPERCASE), structs de `equipment.json` (todas as categorias + passives + warbond + stratagemInfo, espelhando as chaves), `SUPPORT_STRATS` (R12). `stratagems.json` lido do disco no boot; `equipment.json`/`statsMap.json` atrás de `OnceLock` (primeira visita à aba Builds). `EXCLUSIVE_TAGS`, `has_exclusive_conflict()`, `normalize_text()` — portar de `legacy/src/renderer/lib/build.js`. Testes: conflito Mecha/Vehicle, normalize sem acento.
4. `i18n.rs`: struct `Tr` com TODOS os campos de `legacy/src/renderer/data/translations.js` (pt e en, verbatim) + strings hardcoded em JSX com ternário de idioma (caçar `settings.language === 'pt'` no legado: "Atalho do Overlay", "HUD Persistente", modal de update etc.). Remover chaves do boot intro (feature cortada).
5. `keys.rs`: tabelas nome↔VK↔scancode (R7) + testes de ida-e-volta.
6. `shared.rs`: R4 completo, instanciado no boot.

**Critérios de aceite:** `cargo test` verde no host; check windows limpo.

**Commits:**
1. `feat(core): settings with v1 migration and atomic persistence`
2. `feat(core): game data models, shared rules and key tables`
3. `feat(core): full pt/en i18n port and shared state hub`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 2.

---

### FASE 2 — Motor de macro (a parte perfeita)

**Objetivo:** `engine.rs` completo e benchmarkado. **Nada avança enquanto o timing não estiver provado.**

**Tarefas:**

1. Thread "engine" no boot, `SetThreadPriority(THREAD_PRIORITY_HIGHEST)` (`#[cfg(windows)]`). Loop: `engine_rx.recv()`.
2. Concorrência: `macro_running` com `compare_exchange` — se rodando, descarta e emite `UiEvent::MacroBlocked` + `OverlayCmd::Flash{Blocked}`. **Fila tamanho zero: rejeitar, nunca enfileirar.**
3. `run_sequence`:
   - Guard RAII `TimerRes`: `timeBeginPeriod(1)` no início / `timeEndPeriod` no drop.
   - Guard RAII `KeysDown`: registra scancodes pressionados; `Drop` envia KEYUP de todos em ordem inversa (vale pra panic/erro/aborto).
   - Ordem R8; sleeps com `spin_sleep::SpinSleeper::new(500_000)`.
   - **Aborto por foco**: checar `shared.game_focused` antes de cada press; caiu → return (guard limpa).
   - `UiEvent::MacroTriggered` + `MacroStatus{running:true}` **depois** do press do modificador (input primeiro — regra v1); `MacroStatus{running:false}` no fim.
4. `send_scan(scan, ext, up)` via `SendInput` (R7), um evento por chamada.
5. Jitter `±5ms` com `jitter_floor(v, floor)`.
6. Trait `InputSink` (produção = SendInput; teste = vetor de eventos com timestamps) — permite testar ordem exata da sequência e clamps no host.
7. `src/bin/timing_bench.rs`: N sequências turbo (W/A/S/D), timestamps via `QueryPerformanceCounter` em cada SendInput, imprime p50/p95/p99/max de desvio de hold/gap. Meta: p99 < 1ms. Instruções de uso impressas no `--help` (rodar com editor de texto focado).
8. `src/bin/soak.rs`: 1.000 execuções em série (gap 300ms), relatório de violações (desvio > 5ms). Protocolo in-game documentado no cabeçalho (HD2 borderless, 60fps e 30fps cap, zero estratagema falhado).

**Critérios de aceite:** testes de sequência/clamp/ordem verdes no host; bins compilam pra Windows; clippy limpo.

**Commits:**
1. `feat(engine): dedicated macro thread with scancode sendinput and raii guards`
2. `feat(engine): speed profiles, humanized jitter and frame-floor hold`
3. `feat(engine): timing benchmark and soak test harnesses`

> ✅ Commitar, **recomendar `/clear`**. Avisar o usuário: rodar `timing_bench` e `soak` num Windows real antes de confiar no motor.

---

### FASE 3 — Hooks: hotkeys + foco (event-driven)

**Objetivo:** teclas disparam o engine com jogo em foco; foco instantâneo; modo gravação.

**Tarefas:**

1. `hooks.rs` (`#[cfg(windows)]`): thread "hooks" com:
   - `SetWindowsHookExW(WH_KEYBOARD_LL)`: callback lê `vkCode`; se `game_focused && !recording`, procura numa tabela de bindings pré-resolvida (`RwLock<Vec<Binding>>` com `{vk, slot, support, codex, modifier, speed, use_arrows}` — TUDO resolvido na reconstrução, o callback não faz lookup pesado); casou → `engine_tx.send(...)` + retorna 1 (engole). `Ctrl+H` (VK_H + `GetAsyncKeyState(VK_CONTROL)`) com `enableOverlay` → `OverlayCmd::Toggle` + engole. Resto → `CallNextHookEx`.
   - `SetWinEventHook(EVENT_SYSTEM_FOREGROUND, ..., WINEVENT_OUTOFCONTEXT | WINEVENT_SKIPOWNPROCESS)`: `GetForegroundWindow` + `GetWindowTextW`, classifica (R10), atualiza `game_focused`, posta `UiEvent::GameFocus` + transições (R10): `OverlayCmd::SetState`, `Reassert`, `FullscreenWarning`, sinal de update adiado (consumido na Fase 10).
   - `GetMessageW` pump; estado inicial classificado no boot; `SetTimer` 5s revalidando foreground (fallback de evento perdido).
2. `rebuild_bindings(shared)`: chamada no boot e em mudança de settings/slots.
3. Modo gravação: `recording=true` → hook repassa tudo e não dispara.
4. Teste manual documentado no topo do arquivo (Notepad renomeado "HELLDIVERS 2 test": F1 dispara, F1+Shift/Ctrl seguros dispara, tecla engolida, alt-tab desliga na hora).

**Critérios de aceite:** compila pros dois alvos; callback sem alocação e sem bloqueio (justificar `RwLock::read` em comentário — escrita raríssima).

**Commits:**
1. `feat(hooks): low-level keyboard hook with prebuilt binding table`
2. `feat(hooks): event-driven focus watcher with transition state machine`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 4.

---

### FASE 4 — Janela principal + toolkit D2D

**Objetivo:** janela win32 abrindo com tema HUD, toolkit de widgets funcionando, barra de abas navegável (conteúdos placeholder). Fundação de TODA a UI.

**Tarefas:**

1. `gfx/d2d.rs`: `ID2D1Factory1` + `IDWriteFactory` singletons; criação de `ID2D1HwndRenderTarget` (com recriação em device-lost) e de `ID2D1DCRenderTarget`+DIB premultiplicado (pro overlay, Fase 9); brushes sólidos cacheados por cor.
2. `gfx/text.rs`: coleção DirectWrite custom com Inter de `assets/fonts/` (font set builder a partir de arquivos — sem instalar no sistema); `text_format(weight, size)` cacheado; helper `draw_text` com letter-spacing (via `IDWriteTextLayout` quando o tema pedir tracking).
3. `gfx/images.rs`: decode WebP (`image`) → BGRA premultiplicado → `ID2D1Bitmap` via `CreateBitmap`; cache por caminho; bitmaps são por-render-target — invalidar cache em device-lost.
4. `ui/theme.rs`: R5 + `Scale` (DPI) com helpers `dip(f32) -> f32`.
5. `ui/toolkit.rs` (R14): árvore de widgets (`Vec<Widget>` com rects), layout por linhas/colunas/grid com gap/padding, hit-test, estados hover/pressed por widget-id, invalidação, scroll container (wheel + thumb 5px estilizado), gerenciamento de `SetTimer` de animação (criar ao iniciar flash/fade, matar ao terminar).
6. `ui/widgets.rs` (primeira leva): `tab_bar` (3 abas com underline/glow ativo ciano/amarelo, divisores 1px), `hd_card` (fundo CARD_BG, borda, raio 16, borda-esquerda 4px colorida opcional, header com indicador circular + glow), `button` (variantes: primária amarela, secundária escura com hover, chip), `toggle_row` (switch do legado), `edit_host` (EDIT filho skinnado — R14).
7. `ui/window.rs`: window class `MacroHelldivers2Main`, título `Macro Helldivers 2`, ícone (icon.ico do recurso), 820×640 default, restauração/validação de `window-bounds.json` + debounce 500ms em `WM_MOVE`/`WM_SIZE` (`SetTimer`), `WM_DPICHANGED`, `WM_PAINT` → desenha árvore, `WM_APP_UI_EVENT` → drena `ui_rx` e invalida o afetado.
8. `main.rs`: ordem de boot — single instance (Fase 10 completa; aqui só o mutex), shared, threads engine+hooks, janela, message loop. `is_quitting` etc. preparados.

**Critérios de aceite (em Windows/VM):** janela abre < 300ms com tema fiel (fundo #020617, tabs com glow), abas trocam, hover de botões funciona, CPU 0% parado (verificar Task Manager), DPI 125%/150% sem borrão (per-monitor v2).

**Commits:**
1. `feat(gfx): direct2d/directwrite foundation with embedded inter fonts`
2. `feat(ui): d2d widget toolkit with event-driven invalidation`
3. `feat(ui): main window shell with hd theme and tab navigation`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 5.

---

### FASE 5 — Aba Macros completa

**Objetivo:** paridade com a aba de macros da v1.

**Referência:** `legacy/src/renderer/App.jsx` (~460–535, ~900–935), `StratagemCard.jsx`, `Slot.jsx`.

**Tarefas:**

1. `ui/widgets.rs` (segunda leva): `slot_square` (64×64: vazio/ocupado/ativo/flash-amarelo-500ms/anel-vermelho-400ms, label do atalho no topo, × de limpar no hover), `arrow_row` (setas do codex desenhadas com Painter D2D — triângulo+haste, ciano, 13–16px conforme len>6), `stratagem_card` (aspect 1:1, ícone full-bleed com opacidade 70→100% no hover + leve zoom, gradiente escuro topo com nome, codex embaixo, estados: desabilitado 20% opacidade, equipado-no-slot-ativo borda amarela, hover borda na cor da categoria).
2. `ui/macro_tab.rs`:
   - Header "SELECIONE PARA O SLOT {atalho}".
   - Busca (`edit_host`) com `normalize_text`; seções por primeira tag (ordem Offensive→Supply→Defensive→alfabético; cores RED/CYAN/GREEN em indicador e borda-esquerda); seção some vazia; mensagem "sem resultados".
   - Grade 4 colunas em scroll container.
   - Clique: equipa no slot ativo (regras: duplicado noutro slot → desabilitado; conflito exclusividade → desabilitado; clicar no equipado do slot ativo → remove), avança slot ativo até 3, persiste, `rebuild_bindings`, `OverlayCmd::Slots`.
   - Barra de slots fixa no rodapé (4 `slot_square`, seleção por clique).
3. `loadouts.rs`: `slots.json` com re-resolução por id + limpeza de conflitos (R9).
4. Flashes ligados aos `UiEvent`s (suporte incluído — usados na Fase 6 também).

**Critérios de aceite:** fluxo equipar/trocar/limpar idêntico à v1 (conferir lado a lado com o legado rodando); busca sem acento; persistência entre execuções; scroll suave; flashes.

**Commits:**
1. `feat(ui): stratagem card, slot and codex arrow widgets`
2. `feat(ui): macro tab with grid, search and slot assignment rules`
3. `feat(ui): persistent slots with trigger and blocked feedback`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 6.

---

### FASE 6 — Aba Configurações + captura de tecla + backup

**Objetivo:** paridade da aba Configurações (sem sprint modifier — removido; sem seção de update — Fase 10).

**Referência:** `legacy/src/renderer/App.jsx` (~558–898).

**Tarefas:**

1. `ui/settings_tab.rs`:
   - **Atalhos principais**: 4 botões de captura → `recording=true`, botão pulsa "OUVINDO..." (timer de animação); captura via `WM_KEYDOWN`/`WM_SYSKEYDOWN` da janela principal → nome canônico (R7), ignora modificadores puros, Esc cancela; salva, `recording=false`, `rebuild_bindings`.
   - **Jogo/modificadores**: modificador in-game (CTRL/ALT/=/−), velocidade (3 perfis + descrições), toggle WASD↔Setas, toggle "Atalho do Overlay" (enableOverlay — sobe/derruba thread overlay + `SetState(Hidden)`), toggle "HUD Persistente" (alwaysShowSlots — com jogo focado alterna minimal/hidden na hora, portar `legacy/src/main/index.js` ~737–746).
   - **Idioma**: pt/en aplicado na hora (invalidate geral).
   - **Apoios fixos**: 3 cards (ícone webp, nome, codex) + captura de atalho cada (`support-{i}`), flashes via `UiEvent`.
   - **Backup**: export/import via `IFileDialog` COM (`util.rs`: `save_dialog(default_name, filter)` / `open_dialog(filter)`), formato R9, status flash 2,5s (verde/vermelho).
   - **Rodapé**: versão (`CARGO_PKG_VERSION`), bolinha verde/vermelha SISTEMA ATIVO/INATIVO (game_focused); espaço reservado pro updater (Fase 10).
2. Toda mudança: save + `rebuild_bindings` + `OverlayCmd` pertinente.

**Critérios de aceite:** captura com F-keys/numpad/setas/Esc; toggles refletem na hora (HUD persistente aparece/some com jogo focado); backup round-trip com fixture v1 real (`tests/fixtures/backup-v1.json` — criar representativa); idioma troca tudo.

**Commits:**
1. `feat(ui): settings tab with key capture and modifier options`
2. `feat(ui): fixed support stratagem bindings`
3. `feat(ui): json backup export/import via native file dialogs`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 7.

---

### FASE 7 — Builds: lógica pura + Aleatória, Personalizada, Salvas

**Objetivo:** aba Builds sem a sub-aba Meta (Fase 8). Toda geração em `builds.rs` PURA (testável no host).

**Referência:** `legacy/src/renderer/components/BuildTab.jsx` — portar algoritmos LINHA A LINHA em comportamento.

**Tarefas:**

1. `data.rs`: `strat_meta` — casamento estratagema↔stratagemInfo **exato** (~70–101): normalização `[^a-z0-9]`, match exato primeiro, substring só ≥6 chars ficando com o candidato mais longo, fallback por designação (1º token, ≥4 chars). Predicados `is_support/is_backpack/is_sentry`.
2. `builds.rs`:
   - `Build { stratagems: [Option<u32>;4], equip: HashMap<Slot, String> }`, `BuildLocks`.
   - `gen_random(prev, locks, settings, data)` — portar `generateFullBuild` (~168–212): locks, `can_add` (duplicado/exclusividade/maxOneSentry/balanced), balanced coloca 1 apoio + 1 mochila primeiro, resto de pool embaralhada, equipamento aleatório com locks, `apply_set_matching` (~156–166: capacete de nome idêntico; capa da mesma warbond aleatória entre candidatas).
   - Testes: balanced garante apoio+mochila; maxOneSentry; locks; sem duplicata; sem 2 Mecha/Vehicle.
3. `ui/widgets.rs`: `dropdown` (R14 — popup D2D com lista rolável) e `build_item_card` (imagem, label ciano, badge SET amarela, cadeado 🔒/🔓 clicável no canto, subtítulo/descrição).
4. `ui/build_tab.rs`:
   - Sub-tabs Meta/Aleatória/Personalizada (Meta placeholder nesta fase).
   - Card de opções (matchSet/balanced/maxOneSentry) fora da Personalizada; gravam em settings.
   - **Aleatória**: botão "🎲 Gerar Build" + hint.
   - **Personalizada** (~490–608): 4 slots clicáveis em edição, grade 5 colunas com busca (reuso do `stratagem_card` com assign custom), remoção por clique/×, avanço de slot, dropdowns de equipamento com "Nenhum", botões "Usar slots atuais" e "Limpar tudo".
   - **Exibição da build**: 4 `build_item_card` de estratagema + grade de equipamento (armas: tipo · dano; armadura: peso + ARM/VEL/STA + passiva com descrição via passiveByName; booster: descrição; badge SET), locks funcionais, botão "Aplicar estratagemas nos slots" (validação de conflito + persiste).
   - **Salvas**: chips (aplicar — portar `handleApplyLoadout` ~130–148 com os 3 casos de builds antigas; excluir no hover; destaque da ativa), input nome + salvar (dedupe case-insensitive sobrescreve; default `Build {n}`), `loadouts.json`.

**Critérios de aceite:** testes de `builds.rs` verdes no host; fluxo aleatória→lock→re-gerar→salvar→aplicar→excluir idêntico à v1; personalizada completa; dropdowns fecham direito (clique-fora/Esc).

**Commits:**
1. `feat(builds): pure generation logic with balance, sentry and set rules`
2. `feat(ui): dropdown and build item card widgets`
3. `feat(ui): build tab with random and custom builders`
4. `feat(ui): saved loadouts with apply, overwrite and delete`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 8.

---

### FASE 8 — Builds: sub-aba Meta (helldive.live)

**Objetivo:** paridade da sub-aba Meta.

**Referência:** `legacy/src/renderer/components/BuildTab.jsx` (~283–424, ~624–728) e R13.

**Tarefas:**

1. `meta_stats.rs`: `fetch(faction, difficulty)` em worker thread (ureq, timeout 10s, 3 requests), structs serde tolerantes (`loadouts_percentage: f64`, `change: Option<f64>`, `isNew: Option<bool>`, `total.games`), cache `meta-cache.json` TTL 6h, resultado via `UiEvent::MetaStats` (loading/erro/dados).
2. `builds.rs`: `meta_lists()` (slugs→itens via statsMap, ordenação por pick rate, categorias, passivas) e `gen_meta(...)` — portar `generateMetaBuild` exato: `TOP_STRATS=10`, `weighted_from`, fallback top-3 da lista completa, armas top-3/categoria, armadura via top-5 passivas→candidatas, helmet/cape/booster aleatórios com locks, `apply_set_matching`. Testes: proporção de pesos (estatístico grosseiro), fallback de mochila.
3. `ui/build_tab.rs`: sub-aba Meta — facção (3) × dificuldade (All/7/8/9/10), estados loading (pulso)/erro/dados, botão "🏆 Gerar Build Meta", listas: top 10 estratagemas (ícone, nome, NOVO, Δ ▲▼ colorido, barra proporcional ao 1º, %), top 3 armas/categoria, top 5 passivas, crédito "helldive.live · N partidas".

**Critérios de aceite:** com rede funciona; sem rede degrada com cache/erro; testes verdes.

**Commits:**
1. `feat(meta): helldive.live stats client with 6h disk cache`
2. `feat(builds): weighted meta build generation from top picks`
3. `feat(ui): meta sub-tab with faction and difficulty stats`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 9.

---

### FASE 9 — Overlay: strip + painel (D2D layered)

**Objetivo:** overlay completo custo-zero: strip estático e painel enxuto interativo, sem jamais roubar foco.

**Tarefas:**

1. `overlay/strip.rs` (`#[cfg(windows)]`), thread "overlay":
   - Window class com título **`HD2_OVERLAY`** (R10), estilos R11 (strip com `WS_EX_TRANSPARENT` permanente), `ShowWindow(SW_SHOWNOACTIVATE)` uma vez, NUNCA esconder.
   - Render: DIB premultiplicado + `ID2D1DCRenderTarget` + `UpdateLayeredWindow` (`ULW_ALPHA`). **Redesenho SÓ em mudança** (slots/flash/state); sem loop.
   - Conteúdo minimal: fundo arredondado translúcido, 4 slots 64px×0,70 com ícone (via `gfx/images`), label do atalho, flash amarelo 500ms / vermelho 400ms (`SetTimer` local).
   - Bounds por estado (R11) com DPI do monitor primário; timer 5s de reassert enquanto visível; reassert imediato em `Reassert`; `WM_DISPLAYCHANGE`/`WM_DPICHANGED` recalculam.
   - Canal: `WM_APP_OVERLAY` postado pelos senders; drenagem no WndProc.
2. `overlay/panel.rs` (mesma thread): segunda janela layered, título `HD2_OVERLAY`, SEM `WS_EX_TRANSPARENT` quando visível como painel (recebe mouse; `WS_EX_NOACTIVATE` impede roubo de foco). 840×660 central, mesmo pipeline DIB/`UpdateLayeredWindow`, **usando `ui/toolkit.rs`** (targets compartilham trait):
   - Container "vidro": fundo `#020617` ~85% alpha, raio 24, borda branca 10%.
   - Header: botão × (fecha = `OverlayCmd::Toggle`) + aviso fullscreen quando ativo.
   - **Conteúdo enxuto (decisão de escopo)**: duas seções — (a) slots de macro + grade de estratagemas com busca pra atribuição (mesmos widgets da Fase 5, flag `is_overlay`); (b) builds salvas: chips aplicáveis (sem salvar/nomear/excluir). NADA de configurações, NADA de geração de builds.
   - Mouse: hit-test do toolkit via `WM_MOUSEMOVE`/`WM_LBUTTONDOWN` (janela NOACTIVATE recebe mouse sem ativar); busca usa `edit_host` — RESSALVA v1 mantida: teclado pode não chegar (janela nunca ativa); se `EDIT` não receber input sem ativação, ocultar a busca no painel (paridade: v1 tinha a mesma limitação com captura de teclado no overlay).
3. `overlay/mod.rs`: máquina de estados (R11): `Toggle`: panel→(minimal se alwaysShowSlots senão hidden), senão→panel; transições por foco; `SetState` aplica bounds nas duas janelas + `WS_EX_TRANSPARENT` do painel conforme estado; sobe/derruba thread conforme `enableOverlay`.
4. `game_config.rs`: implementar (R11) + testes da regex com fixtures (fullscreen true/false/borderless).
5. Aviso fullscreen também na janela principal (banner topo) quando `FullscreenWarning(true)` e jogo focado (paridade v1).

**Critérios de aceite (Windows + jogo real):** borderless: Ctrl+H abre painel sem minimizar o jogo nem roubar foco (macros disparam com painel aberto); atribuir slot e aplicar build salva pelo painel funcionam por mouse; HUD persistente sobrevive alt-tab ida/volta; "Tela Cheia" exclusiva mostra aviso; CPU ~0% com strip parado (Process Explorer); flashes visíveis in-game.

**Commits:**
1. `feat(overlay): direct2d layered strip with state-driven redraw`
2. `feat(overlay): interactive no-activate panel with slots and saved builds`
3. `feat(overlay): state machine, hotkey toggle and fullscreen warning`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 10.

---

### FASE 10 — Tray, single instance, updater

**Objetivo:** app residente + ciclo de atualização completo.

**Tarefas:**

1. `tray.rs` (cru): `Shell_NotifyIconW` com `assets/icons/tray.png` (converter pra HICON via decode + `CreateIconIndirect`), tooltip, `WM_APP_TRAY` callback: clique/duplo restaura (`ShowWindow`+`SetForegroundWindow`); menu popup (`CreatePopupMenu`/`TrackPopupMenu`): "Abrir Macro Helldivers 2" / "Sair". Minimizar (`WM_SYSCOMMAND SC_MINIMIZE`) e fechar (`WM_CLOSE`) → esconder janela, app segue (hooks vivos). "Sair" → `is_quitting=true`, destruir ícone, `PostQuitMessage`. Sem tray criado → fechar encerra (proteção v1).
2. Single instance (`util.rs`): `CreateMutexW` global; se existe, `FindWindowW` da classe principal + restaurar + sair.
3. `updater.rs`:
   - `check()`: GET `https://api.github.com/repos/DionathaGoulart/Macro-Helldivers2/releases/latest` (User-Agent), `tag_name` `vX.Y.Z` vs `CARGO_PKG_VERSION` (semver). Estados → `UiEvent::UpdateStatus` (Checking/Available{version}/UpToDate/Downloading{percent}/Ready/Error{msg}).
   - **Nunca com jogo em foco**: primeiro check ~1,5s pós-boot SE não focado; senão flag adiada consumida na perda de foco (sinal da Fase 3; portar semântica v1 `hasCheckedUpdates`).
   - `download()` só por clique: asset `.exe` do release → temp, progresso em chunks.
   - `install()`: `Command` no instalador baixado + encerrar app.
4. UI: rodapé do settings (status + botão BAIXAR quando available) + `ui/modal.rs` "Atualização Disponível / Reiniciar Agora / Depois" quando Ready (textos do i18n, portar do legado).
5. `build.rs` + manifest: `winresource` com `assets/icon.ico`; manifest: `requireAdministrator` + `perMonitorV2`.

**Critérios de aceite:** fechar/minimizar esconde e macros seguem; tray restaura; segunda instância foca a primeira; updater percorre available→download→ready→install com release de teste; exe com ícone e pedindo elevação.

**Commits:**
1. `feat(app): raw win32 tray with hide-to-tray lifecycle and single instance`
2. `feat(updater): github releases check, manual download and install flow`
3. `chore: embed exe icon and admin/dpi manifest`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 11.

---

### FASE 11 — Instalador NSIS + release CI

**Objetivo:** distribuição: instalador < 5MB, release automatizado por tag.

**Tarefas:**

1. `installer/installer.nsi` (MUI2): `RequestExecutionLevel admin`, perMachine em `$PROGRAMFILES64\Macro Helldivers 2`, diretório customizável, atalhos Menu Iniciar + Desktop "Macro Helldivers 2"; instala exe + árvore `assets/`; registro uninstall (DisplayName/DisplayVersion/Publisher/UninstallString); **upgrade da v1**: varrer `HKLM\...\Uninstall\*` por DisplayName "Macro Helldivers 2" do electron-builder e rodar uninstaller antigo `/S` antes (best-effort); uninstaller remove arquivos+registro, PRESERVA `%APPDATA%\Macro Helldivers 2`; opção "Executar ao concluir"; versão via `/DVERSION=`.
2. `.github/workflows/release.yml`: `push tags v*` → `windows-latest`: `cargo build --release`, NSIS (choco), `makensis /DVERSION=...`, SHA256, GitHub Release com instalador + checksum (`gh release create` ou softprops).
3. `ci.yml`: adicionar `cargo build --release` como smoke de linkagem.
4. Validação real no primeiro tag beta (`v2.0.0-beta.1`): instala/desinstala limpo em VM; updater da Fase 10 acha e instala o beta seguinte.

**Commits:**
1. `feat(installer): nsis per-machine installer with v1 upgrade path`
2. `ci: release workflow building installer on version tags`

> ✅ Commitar, **recomendar `/clear`**, próxima: Fase 12 (final).

---

### FASE 12 — QA de paridade, benchmarks, limpeza e release

**Objetivo:** fechar a v2.0.0: paridade validada, métricas medidas, legado removido, docs.

**Tarefas:**

1. **Checklist de paridade** (validar no Windows com o jogo; marcar aqui):
   - [ ] 4 slots com atalhos configuráveis; captura com Esc; recording desliga hook
   - [ ] 3 apoios fixos com atalhos próprios
   - [ ] Perfis normal/fast/turbo; WASD↔Setas; modificador Ctrl/Alt/=/−
   - [ ] Macro dispara com QUALQUER modificador seguro (correr+F1 funciona)
   - [ ] Macro só com jogo/app/overlay focado; feedback disparo/bloqueio
   - [ ] Aborto ao perder foco solta todas as teclas
   - [ ] Overlay: Ctrl+H, hidden/minimal/panel, click-through, sem roubo de foco, HUD persistente, aviso fullscreen, × fecha
   - [ ] Painel enxuto: atribuir slots + aplicar builds salvas por mouse
   - [ ] Aba Macros: categorias/cores, busca sem acento, exclusividade, avanço de slot
   - [ ] Builds Meta: facção×dificuldade, cache 6h, tops com Δ/NOVO, geração ponderada
   - [ ] Builds Aleatória: sets, balanceado, máx 1 sentinela, locks
   - [ ] Builds Personalizada: slots, grade+busca, equipamento, importar, limpar
   - [ ] Salvas: salvar/sobrescrever/aplicar/excluir/destaque
   - [ ] Backup: export/import compatível com arquivo da v1
   - [ ] Tray: minimizar/fechar esconde; Sair encerra; single instance
   - [ ] Updater: check adiado com jogo focado; download manual; instalar
   - [ ] i18n pt/en completo; bounds da janela persistem; DPI 100/125/150%
2. **Benchmarks** (tabela no CHANGELOG):
   - RAM (private bytes): idle < 20MB; overlay ativo < 22MB
   - CPU idle com jogo focado: ~0%
   - Boot até janela útil: < 300ms
   - `timing_bench`: p99 < 1ms; `soak` in-game: 1.000 turbo @60fps e @30fps → 0 falhas
   - PresentMon: frametime HD2 com HUD persistente on/off → delta < 0,2ms médio
   - Instalador < 5MB
3. **Limpeza**: `git rm -r legacy/`; ajustar `.gitignore`.
4. **Docs**: `README.md` reescrito (stack win32/Rust, features, dev com cargo-xwin, pipeline de dados, seção overlay/fullscreen mantida, mudanças deliberadas documentadas); `CHANGELOG.md` entrada `2.0.0` (reescrita, remoções deliberadas com justificativa, métricas antes/depois); `Cargo.toml` → `2.0.0`.
5. Tag e release: **aguardar comando do usuário** (não taguear sem pedir).

**Commits:**
1. `fix: ...` (um por bug achado no QA, mensagem própria)
2. `chore: remove legacy electron implementation`
3. `docs: rewrite readme and changelog for the native 2.0.0 release`
4. `chore: bump version to 2.0.0`

> ✅ Fim do plano. Recomendar `/clear` e revisão humana antes do tag.

---

## RISCOS E MITIGAÇÕES

| Risco | Mitigação |
|---|---|
| GameGuard tratar SendInput de exe novo diferente | Mesma classe userland da v1 (nut.js usava SendInput); testar in-game na Fase 2 com `soak`; jitter mantido; zero toque no processo do jogo |
| LL hook × software de teclado (Razer/Logitech) | Matriz de teclas na Fase 3; fallback `RegisterHotKey` atrás de feature flag se necessário |
| `EDIT` no painel NOACTIVATE sem teclado | Ressalva já aceita (v1 tinha limitação equivalente); ocultar busca no painel se confirmar |
| DIB/UpdateLayeredWindow com DPI misto | Overlay sempre no monitor primário (igual v1); `WM_DPICHANGED` recalcula |
| Toolkit D2D subestimado (dropdown/scroll/edit) | Fase 4 dedicada só a ele; dropdown/edit validados na Fase 7/6 antes do overlay reutilizar |
| Device-lost do D2D (troca de GPU, driver) | Recriação de render target + invalidação do cache de bitmaps centralizadas em `gfx/d2d.rs` desde a Fase 4 |
| `panic = "abort"` em release vs guards RAII | Guards protegem unwind (dev/testes); em release, abort derruba o processo e o SO descarta o estado sintético — aceitável |
| Slots/loadouts v1 (localStorage) não migram | Caminho suportado: exportar backup na v1, importar na v2 (formato idêntico); documentar no README e release notes |

## FORA DE ESCOPO

- Nenhuma feature nova de gameplay — reescrita com paridade, exceto as **mudanças deliberadas** listadas no topo.
- macOS: sem alvo de build (dev/check/test de lógica seguem no host via stubs).
- Pipeline da wiki permanece em Node (dev-only).
