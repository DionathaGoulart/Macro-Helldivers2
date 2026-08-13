# Changelog

Todas as mudanças relevantes deste projeto são documentadas aqui.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
e o versionamento segue [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [Não lançado]

Sincronização com a wiki da comunidade após a warbond **Castellan's Creed**
(Helldivers 2 × Warhammer 40.000, 12/08/2026), mais uma passada de desempenho
ponta a ponta em CPU, RAM e GPU.

### Desempenho

- **O overlay não ocupa mais a tela inteira o tempo todo.** A janela transparente
  continua viva (escondê-la roubaria o foco do jogo), mas agora muda de tamanho
  conforme o estado: 1×1 quando não há nada a mostrar, o retângulo do strip no modo
  minimal e 840×660 no painel. Antes o DWM compunha uma camada do tamanho do monitor
  por cima do jogo em todo frame, mesmo sem nada desenhado.
- **Scanline e granulado deixaram de vazar pro jogo.** Os dois pseudo-elementos de
  tela cheia do `body` eram pintados também na janela de overlay, permanentemente.
- **Menos camadas de composição e desfoque**: removidos o `will-change` das ~91
  imagens de estratagema (cada uma virava uma layer de GPU permanente por causa do
  hover) e o `backdrop-blur` dos cards, do header e do rodapé, onde o fundo é opaco
  e o desfoque não mudava um pixel.
- **Detecção de foco mais barata**: o título da janela ativa só é lido quando o handle
  muda, o intervalo cai pra 2 s quando o jogo não está em foco, e o z-order do overlay
  passou a ser reafirmado a cada ~5 s em vez de duas vezes por segundo.
- **Boot do processo principal**: `electron-updater` (com js-yaml, semver e lodash
  junto) saiu do bundle e virou carregamento sob demanda — 253 KB → 12 KB.
- **Aba de builds em chunk separado**: `equipment.json` e `statsMap.json` só são
  baixados na primeira visita. O bundle inicial do renderer caiu de 369 KB para 254 KB.
- **Imagens em WebP**: 429 PNG convertidos com as mesmas dimensões, 17,7 MB → 3,0 MB,
  via o novo `npm run optimize-images`. O ícone do tray virou um `tray.png` de 64 px
  em vez do `icon.png` de 1024 px.
- **Atualização não baixa mais sozinha durante a partida**: a checagem é adiada
  enquanto o jogo está em foco e o download passou a exigir confirmação.
- `spellcheck: false`, `sandbox`, `v8CacheOptions` e `backgroundThrottling` explícitos
  nas duas janelas.

### Adicionado

- **Estratagema 40-K Meltagun** (`⬇️⬅️⬆️⬅️⬅️⬇️`), da Castellan's Creed. A wiki ainda não
  publicou o ícone próprio, então ele usa o genérico de arma de apoio por enquanto.
- **Warbonds no arsenal**: `equipment.json` agora traz as 24 warbonds com capa, tipo
  (Padrão / Premium / Lendária), data de lançamento e preço.
- **Armadura KDM-500 Outrider** e a passiva **Kinetic Displacement Mitigation**.
- **`npm run sync-stratagems`**: script que sincroniza `stratagems.json` e os ícones
  de `public/stratagems/` com a wiki, casando por código de entrada (imune a rename)
  e preservando os IDs — builds e slots salvos continuam válidos.

### Alterado

- **Ícones dos estratagemas redesenhados**: a wiki migrou os ícones para SVG com paleta
  nova e moldura colorida por tipo de permissão (vermelho / ciano / verde), que agora
  bate com as cores das categorias na interface. Todos os 91 foram regerados.
- **Estratagemas renomeados** conforme o jogo: os drones perderam a alcunha "Guard Dog"
  (`AX/LAS-5 Rover`, `AX/ARC-3 K-9`, `AX/FLAM-75 Hot Dog`, `AX/TX-13 Dog Breath`,
  `AX/AR-23 Guard Dog`), `M-102 Fast Recon Vehicle` virou `M-102 Gunner FRV` e
  `SH-20 Ballistic Shield Supply` virou `SH-20 Ballistic Shield Backpack`.
- **Nomes das armas 40-K corrigidos** (`R/40-K Hot-Shot Marksman Rifle`,
  `P/40-K Bolt Pistol`, `G/40-K Melta Mine`) e a capa `United in Equality` perdeu o
  sufixo `(Cape)`.

### Corrigido

- **Perfis de velocidade que perdiam input.** O jogo lê o teclado uma vez por quadro
  (16,7 ms a 60 fps), e os perfis Rápida e Turbo seguravam a tecla por 15 ms e 10 ms —
  o jitter de ±5 ms chegava a derrubar o Turbo para 5 ms. Uma tecla que sobe e desce
  entre dois quadros não existe pro jogo, e o estratagema falhava de forma
  intermitente. Agora cada perfil tem um piso de tempo de tecla medido em quadros
  (e é rotulado pelo FPS que garante), e a velocidade vem do intervalo entre teclas.
- **Latência do disparo**: a sequência de teclas começa antes dos avisos de IPC pra
  interface, não depois.
- **Casamento de metadados da wiki**: nomes curtos podiam casar com o estratagema
  errado por substring, contaminando as regras de build balanceada.
- **Mapeamento de estatísticas quebrado pelos renames**: os slugs `guard_rover`,
  `guard_arc`, `guard_hot`, `guard_breath` e `backpack_ballistic` deixaram de casar
  com os nomes novos e sumiam do modo Meta. Corrigidos na tabela de apelidos.
- **Imagens órfãs em `public/equipment/`**: oito arquivos com `39` no nome (resíduo de
  `&#39;` mal decodificado em uma extração antiga) foram removidos.

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

[1.0.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v1.0.0
