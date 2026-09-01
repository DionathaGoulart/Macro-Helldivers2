# Changelog

Todas as mudanças relevantes deste projeto são documentadas aqui.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
e o versionamento segue [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [2.0.0] - 2026-09-01

**Reescrita completa.** O app deixou de ser um Electron com React e virou um binário
nativo em **Rust sobre Win32 puro**: um processo, janela e overlay desenhados em
Direct2D, entrada por `SendInput` com scancode, atalhos por `WH_KEYBOARD_LL` e foco
por `SetWinEventHook`. Não há Chromium, Node, nut.js nem processo separado de
renderer. Toda a funcionalidade da v1 foi portada — as exceções estão em
**Removido**, e são deliberadas.

A entrega inclui também a sincronização com a wiki da comunidade posterior à warbond
**Castellan's Creed** (Helldivers 2 × Warhammer 40.000, 12/08/2026), que nunca chegou
a sair numa versão 1.x.

### Alterado

- **Motor de macro em thread própria** com prioridade elevada, `timeBeginPeriod(1)` e
  espera híbrida (`spin_sleep`): a sequência não depende mais do event loop do Node.
  As teclas saem por **scancode** em vez de virtual-key, que é o que o jogo lê.
- **Guards de liberação de tecla**: modificador e direção são soltos por RAII, então
  perder o foco no meio de uma sequência (ou qualquer erro) nunca deixa uma tecla
  presa no jogo.
- **Foco por evento, não por polling**: `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`
  substituiu o timer que lia o título da janela ativa em intervalo fixo. Alt-tab
  desarma os atalhos na hora e aborta a sequência em andamento.
- **Interface sem loop de render**: a janela e o overlay só repintam em mudança de
  estado — clique, evento recebido ou animação em curso. Parado, o app não desenha.
- **Overlay dimensionado por estado**: a janela transparente continua sempre viva
  (escondê-la roubaria o foco do jogo), mas mede 1×1 quando não há nada a mostrar,
  o retângulo do strip no modo minimal e 840×660 no painel. Antes o compositor
  empilhava uma camada do tamanho do monitor sobre o jogo em todo frame.
- **Atualização não baixa mais sozinha durante a partida**: a checagem fica adiada
  enquanto o jogo está em foco e o download exige confirmação.
- **Dados de equipamento carregados sob demanda**: `equipment.json` e `statsMap.json`
  só são lidos na primeira visita à aba Builds.
- **Ícones em WebP**: 429 PNG convertidos com as mesmas dimensões, 17,7 MB → 3,0 MB.
  O ícone da bandeja é um `tray.png` de 64 px em vez do `icon.png` de 1024 px.
- **Instalador NSIS per-machine** no lugar do electron-builder, com desinstalação
  automática da v1 antes de instalar e preservação de `%APPDATA%\Macro Helldivers 2`.
- **Ícones dos estratagemas redesenhados**: a wiki migrou para SVG com paleta nova e
  moldura colorida por tipo de permissão (vermelho / ciano / verde), que agora bate
  com as cores das categorias na interface. Todos os 91 foram regerados.
- **Estratagemas renomeados** conforme o jogo: os drones perderam a alcunha "Guard Dog"
  (`AX/LAS-5 Rover`, `AX/ARC-3 K-9`, `AX/FLAM-75 Hot Dog`, `AX/TX-13 Dog Breath`,
  `AX/AR-23 Guard Dog`), `M-102 Fast Recon Vehicle` virou `M-102 Gunner FRV` e
  `SH-20 Ballistic Shield Supply` virou `SH-20 Ballistic Shield Backpack`.
- **Nomes das armas 40-K corrigidos** (`R/40-K Hot-Shot Marksman Rifle`,
  `P/40-K Bolt Pistol`, `G/40-K Melta Mine`) e a capa `United in Equality` perdeu o
  sufixo `(Cape)`.

### Adicionado

- **Estratagema 40-K Meltagun** (`⬇️⬅️⬆️⬅️⬅️⬇️`), da Castellan's Creed. A wiki ainda não
  publicou o ícone próprio, então ele usa o genérico de arma de apoio por enquanto.
- **Warbonds no arsenal**: `equipment.json` traz as 24 warbonds com capa, tipo
  (Padrão / Premium / Lendária), data de lançamento e preço.
- **Armadura KDM-500 Outrider** e a passiva **Kinetic Displacement Mitigation**.
- **`npm run sync-stratagems`**: sincroniza `stratagems.json` e os ícones com a wiki,
  casando por código de entrada (imune a rename) e preservando os IDs — builds e
  slots salvos continuam válidos.
- **Aviso de erro no boot**: se a instalação estiver sem a pasta `assets/`, o app
  explica em uma caixa de diálogo em vez de morrer em silêncio.

### Removido

Decisões da reescrita, não regressões:

- **Opção "modificador de sprint"**. O hook de teclado de baixo nível dispara com
  qualquer modificador pressionado, então correr e chamar um estratagema já funciona
  sem configurar nada — a opção não tinha mais o que resolver.
- **Animação de abertura** (intro CRT de 4,5 s). O app abre direto na interface.
- **Configurações e criação de builds dentro do overlay.** O painel ficou com o que
  se faz de mouse no meio da partida: atribuir slots e aplicar builds salvas. O resto
  vive na janela principal, que é onde o teclado chega — o overlay não recebe teclado
  por design, e era o que garantia que ele nunca roubasse o foco do jogo.

### Corrigido

- **Perfis de velocidade que perdiam input.** O jogo lê o teclado uma vez por quadro
  (16,7 ms a 60 fps), e os perfis Rápida e Turbo seguravam a tecla por 15 ms e 10 ms —
  o jitter de ±5 ms chegava a derrubar o Turbo para 5 ms. Uma tecla que sobe e desce
  entre dois quadros não existe pro jogo, e o estratagema falhava de forma
  intermitente. Agora cada perfil tem um piso de tempo de tecla medido em quadros
  (e é rotulado pelo FPS que garante), e a velocidade vem do intervalo entre teclas.
- **Latência do disparo**: a sequência de teclas começa antes dos avisos para a
  interface, não depois.
- **Casamento de metadados da wiki**: nomes curtos podiam casar com o estratagema
  errado por substring, contaminando as regras de build balanceada.
- **Mapeamento de estatísticas quebrado pelos renames**: os slugs `guard_rover`,
  `guard_arc`, `guard_hot`, `guard_breath` e `backpack_ballistic` deixaram de casar
  com os nomes novos e sumiam do modo Meta. Corrigidos na tabela de apelidos.
- **Imagens órfãs no arsenal**: oito arquivos com `39` no nome (resíduo de `&#39;` mal
  decodificado em uma extração antiga) foram removidos.
- **Console preto ao abrir o app**: o executável de release passou a ser linkado no
  subsistema `windows`.

### Migração da v1

Configurações são migradas na primeira execução. **Slots e builds salvas não** —
viviam no armazenamento interno do Chromium, que não existe mais. O caminho é
exportar o backup JSON na v1 e importá-lo na v2; o formato do arquivo é o mesmo.

### Desempenho

Metas de projeto da reescrita, e como cada uma é verificada:

| Métrica | Meta | Como medir |
|---|---|---|
| RAM (private bytes), parado | < 20 MB | Gerenciador de Tarefas / Process Explorer |
| RAM com overlay ativo | < 22 MB | idem |
| CPU parado, com o jogo em foco | ~0% | Process Explorer |
| Boot até a janela útil | < 300 ms | cronômetro a partir do clique |
| Desvio de timing das teclas | p99 < 1 ms | `cargo run --bin timing_bench` |
| Falhas de estratagema em 1.000 turbo | 0 | `cargo run --bin soak`, in-game a 60 e 30 fps |
| Impacto no frametime do jogo (HUD ligado) | < 0,2 ms médio | PresentMon, com e sem overlay |
| Tamanho do instalador | < 5 MB | artefato do release |

## [1.0.0] - 2026-08-08 (pré-release)

Primeira versão pública do **Macro Helldivers 2**.

### Adicionado

- **Macros de estratagema**: 4 slots com atalho configurável, executados por emulação
  de hardware (`nut.js`) com intervalos humanizados e três perfis de velocidade
  (Padrão, Rápida e Turbo).
- **Estratagemas de apoio fixos**: Reforço, Ressuprimento e Rearmar Eagle em atalhos
  próprios, independentes dos 4 slots.
- **Detecção de janela**: os macros só disparam com o Helldivers 2 (ou o próprio app)
  em foco.
- **Overlay in-game** (`Ctrl + H`): janela transparente que nunca rouba o foco do jogo,
  com strip de slots opcional sempre visível e aviso quando o jogo está em "Tela Cheia"
  (modo incompatível com overlay de janela).
- **Central de Builds** com três modos:
  - **Meta**: pick rates reais da comunidade (helldive.live) por facção e dificuldade,
    com sorteio ponderado pelo top exibido.
  - **Aleatória**: sorteio completo com regras de sets de armadura, loadout balanceado
    (1 arma de apoio + 1 mochila) e máximo de 1 torreta.
  - **Personalizada**: montagem manual do loadout — escolha do slot, grade de
    estratagemas com busca, equipamento opcional e importação dos slots atuais do macro.
- **Builds salvas**: nomeie qualquer build e aplique nos slots de macro com um clique,
  inclusive pelo overlay.
- **Arsenal completo**: base de dados de armas, armaduras, capacetes, capas, boosters e
  passivas extraída da wiki da comunidade, com ícones offline.
- **Busca de estratagemas** sem acento e sem diferenciar maiúsculas.
- **Backup**: exportação e importação de builds, slots e configurações em JSON.
- **Bandeja do sistema**: minimizar ou fechar recolhe o app, que segue rodando os macros.
- **Auto-update** via GitHub Releases.
- **Idiomas**: português e inglês.

### Corrigido

- **Ícone da bandeja ausente no app instalado**: o `extraResources` copiava o ícone para
  `resources/public/icon.png` enquanto o processo principal procurava em
  `resources/icon.png`. Sem ícone, a criação do `Tray` falhava e a janela escondida ao
  fechar ficava inalcançável. O ícone agora é copiado para o caminho esperado, com
  fallbacks na resolução, e fechar só esconde a janela quando existe bandeja.

### Alterado

- **Boot mais rápido**: o polling de foco (que carrega os módulos nativos do `nut.js` e
  bloqueia o processo principal por cerca de 1s) e a criação da janela de overlay agora
  acontecem depois da primeira pintura da janela principal, eliminando o travamento na
  abertura do app.

[2.0.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v2.0.0
[1.0.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v1.0.0
