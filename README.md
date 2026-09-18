# 🛡️ Macro Helldivers 2 — v2.0.0

Ferramenta de macros para os Estratagemas do Helldivers 2. A v2 é uma reescrita
completa: **binário nativo em Rust sobre Win32 puro**, sem Electron, sem Chromium,
sem runtime de JavaScript. Um processo só, e nada rodando enquanto você não aperta
nada: a interface só repinta quando algo muda e a detecção de foco é por evento do
sistema, não por polling.

![Ícone de Ataque Orbital de Precisão](assets/icons/stratagems/Orbital_Precision_Strike_Stratagem_Icon.webp)

O que mudou em cada versão está no [CHANGELOG](CHANGELOG.md).

## 🚀 Funcionalidades

- **4 slots de macro** com atalho configurável (F1–F12, numpad, setas, letras…),
  em três perfis de velocidade (Padrão, Rápida e Turbo).
- **Estratagemas de apoio fixos**: Reforço, Ressuprimento e Rearmar Eagle em
  atalhos próprios, fora dos 4 slots.
- **Timing à prova de frame**: as teclas são enviadas por scancode via `SendInput`,
  com intervalos humanizados e um piso de tempo de tecla — o jogo lê o teclado uma
  vez por quadro, e uma tecla mais curta que isso simplesmente não existe pra ele.
- **Dispara enquanto você corre**: qualquer modificador seguro pode estar
  pressionado; correr e chamar um estratagema funciona.
- **Só com o jogo em foco**: a detecção é por evento do sistema, não por polling —
  alt-tab desarma os atalhos na hora e uma sequência em andamento é abortada,
  soltando todas as teclas.
- **Overlay in-game** (`Ctrl + H`): janela transparente que nunca rouba o foco,
  com strip de slots opcional sempre visível e um painel para atribuir
  estratagemas e aplicar builds salvas com o mouse.
- **Central de Builds** em três modos: **Meta**, com pick rates reais da comunidade
  (helldive.live) por facção e dificuldade; **Aleatória**, com regras (sets de
  armadura, loadout balanceado, máximo de 1 torreta) e cadeados por item; e
  **Personalizada**, montada na mão.
- **Builds salvas**: nomeie e aplique nos slots com um clique, inclusive pelo overlay.
- **Arsenal completo** offline: 92 estratagemas, armas, armaduras, capacetes, capas,
  boosters, passivas e warbonds, com ícones locais.
- **Busca sem acento** na grade de estratagemas, na mesma ordem dos menus do jogo.
- **Backup** de builds, slots e configurações em JSON — o mesmo formato da v1.
- **Bandeja do sistema**: minimizar ou fechar recolhe o app e os macros seguem ativos.
- **Idiomas**: português e inglês.
- **Tema escuro e claro**, que segue o Windows até você escolher.

## 💻 Requisitos

- **Windows 10 ou 11, 64 bits.** A barra de título pintada nas cores do tema é
  recurso do Windows 11.
- **Permissão de administrador**: o app pede elevação ao abrir, porque o `SendInput`
  precisa do mesmo nível de integridade da janela que recebe as teclas.
- Helldivers 2 em **"Tela Cheia sem Borda"** para usar o overlay (detalhes em
  **Overlay In-Game**, abaixo). Os macros funcionam em qualquer modo de vídeo.

## 📥 Como Instalar e Usar

1. Baixe o instalador `Macro-Helldivers-2-Setup-2.0.0.exe` na aba
   **[Releases](https://github.com/DionathaGoulart/Macro-Helldivers2/releases)**.
2. Execute o instalador e abra o app.
3. Configure seus 4 slots de estratagemas favoritos.
4. No jogo, confira se a tecla que abre o menu de estratagemas é a mesma
   selecionada no app (Ctrl, Alt, `=` ou `-`).
5. **Recomendado**: use as Setas do teclado no app para não interferir no seu
   movimento WASD.

> O app fica na bandeja do sistema: minimizar ou fechar a janela **não encerra** os
> macros. Para sair de vez, use **Sair** no menu do ícone na bandeja.

> O instalador não tem assinatura digital, então o Windows SmartScreen pode avisar
> na primeira execução (**Mais informações → Executar assim mesmo**). Para conferir
> que o arquivo é o publicado, compare o hash com o `.sha256` do mesmo release:
>
> ```powershell
> Get-FileHash .\Macro-Helldivers-2-Setup-2.0.0.exe -Algorithm SHA256
> ```

### Vindo da v1

O auto-update da v1 não enxerga a v2 (o formato do instalador mudou), então baixe o
instalador novo pela aba Releases. Ele desinstala a v1 antes de instalar, e suas
**configurações são migradas automaticamente** (atalhos, tecla do menu, modo setas, velocidade, idioma,
overlay e HUD). **Slots e builds salvas não migram sozinhos** — eles viviam no
armazenamento interno do Chromium, que não existe mais. O caminho:

1. Na v1, aba **Configurações → Backup → Exportar**.
2. Instale a v2 e importe o mesmo arquivo em **Configurações → Backup → Importar**.

## 🔄 Mudanças deliberadas em relação à v1

Não são regressões — são decisões da reescrita:

- **"Modificador de sprint" removido.** O hook de teclado da v2 dispara com qualquer
  modificador pressionado, então a opção não tinha mais o que resolver.
- **Animação de abertura removida.** O app abre direto na interface; boot instantâneo
  vale mais que a intro.
- **Painel do overlay enxuto**: só atribuição de slots e aplicação de builds salvas.
  Configurações e criação de builds ficam na janela principal — que é onde o teclado
  chega de verdade.

---

## 🖥️ Overlay In-Game

O overlay (atalho **Ctrl+H**) é uma janela transparente que **nunca rouba o foco do
jogo** — ele aparece por cima sem minimizar o Helldivers 2. Toda a interação nele é
feita com o mouse.

- **Obrigatório**: use o jogo em **"Tela Cheia sem Borda"** (borderless). No modo
  "Tela Cheia", o Helldivers 2 **se auto-minimiza sempre que qualquer janela desenha
  por cima dele** (comportamento do DXGI fullscreen do jogo, verificado por teste —
  acontece até com janelas nativas do Windows, sem roubo de foco algum). A solução
  usada pelo Discord envolve injeção no processo do jogo — inviável aqui pelo risco
  com o anticheat GameGuard. O app lê a configuração do jogo, detecta o modo "Tela
  Cheia" e exibe o aviso.
- **Limitação**: o overlay não recebe teclado por design (é o que garante que ele não
  roube o foco), então gravar atalhos é feito na janela principal.

---

## 🎯 Builds Personalizadas

Na aba **Builds → Personalizada** você monta o loadout na mão, sem depender de sorteio:

1. Clique no slot (1 a 4) que quer preencher.
2. Clique no estratagema na grade para equipar — clicar de novo no mesmo remove. A
   busca filtra por nome, sem acento.
3. Opcionalmente escolha primária, secundária, granada, armadura, capacete, capa e
   booster nas listas.
4. Dê um nome em **Builds Salvas** e clique em **Salvar**.

Atalhos úteis:

- **Usar slots atuais**: puxa os 4 estratagemas que já estão nos slots do macro.
- **Limpar tudo**: zera a build em edição.
- Clicar em uma build salva aplica os estratagemas direto nos slots de macro
  (funciona também pelo overlay).

As mesmas regras de exclusividade dos sorteios valem aqui: nada de estratagema
repetido nem dois exoesqueletos/veículos na mesma build.

---

## 🎨 Temas

Dois temas no mesmo visual neobrutal: **escuro** (`rose`, o padrão) e **claro**
(`crimson`). Até você escolher, o app segue o modo claro/escuro do Windows. Para
trocar:

- o toggle na barra do topo, ou **`Shift+T`** com a janela do app em foco;
- **Configurações → Tema**, que também tem a opção **Sistema** para voltar a seguir
  o Windows.

O overlay troca junto, e a escolha vai no backup.

---

## 🔄 Atualizações

O app consulta os Releases do GitHub e avisa quando há versão nova.

1. A checagem **nunca acontece com o jogo em foco** — ela fica adiada até você sair
   da partida.
2. O download só começa quando você clica em **Baixar**.
3. Antes de instalar, o app confere o **SHA-256** do instalador contra o `.sha256`
   publicado no release — e confere de novo na hora de executar. Arquivo que não
   bate não é executado.
4. Com o instalador pronto, o app pergunta se quer reiniciar para instalar agora ou
   deixar para depois (o **Instalar agora** continua no rodapé).

---

## 📂 Seus Dados

Tudo fica em `%APPDATA%\Macro Helldivers 2`, que o desinstalador preserva:

| Arquivo | Conteúdo |
|---|---|
| `settings.json` | atalhos, velocidade, idioma, tema e demais opções |
| `slots.json` | os 4 estratagemas dos slots de macro |
| `loadouts.json` | builds salvas |
| `window-bounds.json` | posição e tamanho da janela |
| `meta-cache.json` | estatísticas do helldive.live (valem 6 h) |
| `app.log` | log da sessão atual, recriado a cada abertura |

Se um desses arquivos estiver ilegível, o app o renomeia para `<nome>.bad` e segue
com os padrões — o original não é sobrescrito. Ao relatar um problema, anexe o
`app.log`.

---

## 🛠️ Desenvolvimento e Compilação

O app **só roda no Windows**. Em macOS/Linux dá para compilar, checar e rodar os
testes dos módulos de lógica — todo código de janela, hook e input vive atrás de
`#[cfg(windows)]`. Detalhes em [CONTRIBUTING.md](CONTRIBUTING.md).

```bash
# toolchain (rust-toolchain.toml pina canal, componentes e alvo)
rustup target add x86_64-pc-windows-msvc

# build no Windows
cargo build --release --target x86_64-pc-windows-msvc

# cross-compilar do macOS/Linux, sem Visual Studio
cargo install cargo-xwin
cargo xwin build --release --target x86_64-pc-windows-msvc

# gates de qualidade
cargo fmt --all
cargo clippy --all-targets --target x86_64-pc-windows-msvc -- -D warnings
cargo test
```

Duas bancadas medem o motor de macro no Windows: `cargo run --release --bin
timing_bench` (desvio do relógio, com um editor de texto em foco — ela digita de
verdade) e `cargo run --release --bin soak -- -n 1000 --speed turbo` (mil chamadas
dentro do jogo; o protocolo está no topo de `src/bin/soak.rs`).

O instalador é gerado pela CI a cada tag `vX.Y.Z` (NSIS, em `installer/installer.nsi`),
junto com o `.sha256`; as notas do release saem da seção da versão no
[CHANGELOG](CHANGELOG.md).

### Pipeline de dados (dev-only)

`assets/data/*.json` e `assets/icons/**` são gerados a partir da wiki da comunidade
por scripts Node. Eles não fazem parte do build do app:

```bash
cd scripts
npm install               # sharp + resvg (só aqui; o app não depende de Node)

npm run scrape            # equipment.json + imagens (--refresh ignora o cache)
npm run optimize-images   # PNG → WebP + reescrita das referências
npm run sync-stratagems   # stratagems.json + ícones
npm run stats-map         # statsMap.json (slugs do helldive.live)
```

> A ordem importa: `scrape` deixa `equipment.json` apontando para os `.png` que
> acabou de baixar, e é o `optimize-images` que os converte para WebP e reescreve
> as referências. Rodar um sem o outro deixa o repositório inconsistente.

> Depois de rodar `scrape` ou `sync-stratagems`, rode `stats-map` — ele valida se os
> nomes ainda casam com os slugs do helldive.live e avisa o que ficou sem par.

Nenhum binário externo é necessário: a rasterização de SVG e a conversão para WebP
saem do `sharp` e do `resvg`, instalados pelo `npm install`. O ImageMagick foi
removido do caminho de ícones porque o renderer SVG interno dele descarta elementos
com `transform="rotate(a x y) scale(...)"` — era o que fazia as Eagles Strafing Run
e Napalm Airstrike saírem sem a carga.

---

## 🏗️ Estrutura do Projeto

```text
├── src/
│   ├── main.rs        # boot: instância única, threads, message loop
│   ├── engine.rs      # motor de macro: thread própria, SendInput por scancode
│   ├── hooks.rs       # WH_KEYBOARD_LL + SetWinEventHook (atalhos e foco)
│   ├── focus.rs       # regras de foco do jogo (lógica pura)
│   ├── tray.rs        # ícone e menu da bandeja
│   ├── builds.rs      # geração de builds (lógica pura, testada no host)
│   ├── meta_stats.rs  # cliente do helldive.live com cache em disco
│   ├── settings.rs    # preferências, migração da v1, gravação atômica
│   ├── updater.rs     # GitHub Releases + verificação SHA-256
│   ├── gfx/           # Direct2D, DirectWrite e decode de imagem
│   ├── ui/            # janela principal: toolkit de widgets, tema e abas
│   ├── overlay/       # strip e painel em janelas layered
│   └── bin/           # bancadas timing_bench e soak
├── assets/            # dados, ícones, fontes (JetBrains Mono) e o .ico do exe
├── installer/         # script NSIS
├── scripts/           # pipeline de dados da wiki (Node, dev-only)
├── tests/fixtures/    # backup real da v1 e configs do jogo usados nos testes
└── styleguide.md      # fonte de verdade do visual (tokens, componentes, temas)
```

---

## 🙏 Créditos

- Dados e ícones do jogo: [Helldivers 2 Wiki](https://helldivers.wiki.gg) da comunidade.
- Estatísticas de uso do modo Meta: [helldive.live](https://helldive.live).
- Fonte [JetBrains Mono](https://github.com/JetBrains/JetBrainsMono), sob a SIL Open
  Font License ([`assets/fonts/OFL.txt`](assets/fonts/OFL.txt)).

Projeto de fã, sem vínculo com a Arrowhead Game Studios ou a Sony Interactive
Entertainment. Helldivers é marca de seus respectivos donos.

---

Criado por **DionathaGoulart**. Liberdade ou Morte! ⬆️➡️⬇️⬇️⬇️
