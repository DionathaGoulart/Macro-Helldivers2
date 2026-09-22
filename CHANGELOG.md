# Changelog

Todas as mudanças relevantes deste projeto são documentadas aqui.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
e o versionamento segue [Semantic Versioning](https://semver.org/lang/pt-BR/).

## [2.1.0-beta.1] - 2026-09-22 (instável)

**Versão de teste.** O aviso de atualização da 2.0.0 não a oferece: instale pelo
instalador deste release. Quem ficar nela recebe o aviso da próxima versão estável.

### Adicionado

- **Armas e equipamento novos sem atualizar o app.** A mesma consulta que já trazia
  os estratagemas novos agora traz também armas, armaduras, capacetes, capas,
  boosters e passivas que a [API de dados](https://helldivers-api.dionatha.com.br)
  tem e a versão instalada não. Eles entram nas listas da aba Builds a partir da
  próxima abertura do app, com o ícone e na ordem alfabética da categoria. Mais de
  12 itens novos numa categoria de uma vez é tratado como dado quebrado, e nenhum
  deles entra. Na sub-aba Meta, arma e passiva novas aparecem nos tops assim que o
  helldive.live as registrar.
- **Armas AR-11 Arbitrator, GL-15 Evictor, P-34 Breacher, G-60 Anti-Tank Seeker e
  G-8 Immolation**; armaduras e capacetes **BFM-16 Tanker** e **BFM-220 Ironclad**,
  com a passiva nova **Blunt-Force Mitigation**; capas **Shroud of the Juggernaut** e
  **Standard of Rapid Evacuation**; e a warbond **Ironclad Democracy**.
- **Sub-aba Salvas** na aba de builds. Cada build salva vira uma linha com os quatro
  estratagemas, o equipamento resumido e três ações à vista: **Aplicar** (põe os
  estratagemas nos slots de macro), **Editar** (abre a build na sub-aba
  Personalizada) e **Excluir**. A build que está nos slots ganha a etiqueta
  **Nos slots**.
- **Card Build Atual** em cima da build exibida, com o salvar e o estado dela: nova,
  salva como "X", ou editando "X" com alterações não salvas. Uma build aplicada ou
  aberta para edição fica vinculada à salva: **Salvar alterações** grava por cima
  (trocar o nome no campo renomeia), **Salvar como nova** cria uma cópia e
  **Descartar** volta à versão salva.
- **Rolar de novo** na barra da build atual, nas sub-abas Meta e Aleatória: dá para
  sortear outra vez sem subir até o botão do topo.
- **Animação do sorteio.** Cada card gira como um rolo, passando por outros itens da
  mesma lista, e para no item novo, um depois do outro. Com "Mostrar animações no
  Windows" desligado a build aparece direto.
- Depois de sortear, a página desliza até a build quando ela está fora de vista.
- Toast de confirmação ao salvar, aplicar e excluir uma build.
- **Perfil de velocidade Batata**, para PC que roda abaixo de 30 fps o tempo todo.
  Cada tecla fica segurada 75–85 ms e o intervalo entre direções é de 70–80 ms: os
  dois passam de um quadro de 15 fps mesmo no pior jitter e aguentam até ~13 fps
  (hold) e ~14 fps (intervalo); o menu ganha 250 ms para abrir. Um codex de cinco
  passos leva ~1,2 s.
- **Perfil de velocidade Baixo FPS**, para quem joga travado em 30 fps ou cai abaixo
  disso em combate. Cada tecla fica segurada 45–55 ms e o intervalo entre direções é
  de 40–50 ms: os dois passam de um quadro de 30 fps mesmo no pior jitter, e aguentam
  quedas até ~22 fps (hold) e ~25 fps (intervalo). Um codex de cinco passos leva
  ~0,7 s, contra ~0,4 s do Padrão.
- **Aviso de limite de FPS** em Configurações: o app lê o `max_fps` do
  `user_settings.config` do jogo e, quando o perfil escolhido solta a tecla antes de
  um quadro nesse FPS, sugere o perfil que cobre. Limite imposto por fora do jogo
  (painel da NVIDIA, RivaTuner) não aparece ali. A chave `max_fps` foi lida de
  trechos do arquivo; falta conferir num `user_settings.config` real.
- **Modo Debug** (Configurações → Diagnóstico), desligado por padrão, para
  investigar estratagema que falha num PC que não é o nosso. Ligado, ele grava em
  `debug.jsonl`, uma linha por evento:
  - cada chamada de atalho, com o tempo real de cada tecla contra o planejado
    (lead, hold, intervalo e tail), as teclas que o Windows recusou, as teclas de
    movimento e modificadores que o jogador segurava no disparo e a janela da
    frente; chamadas bloqueadas e sem foco também entram;
  - trocas de foco que armam ou desarmam os atalhos;
  - atalho apertado com o processo do jogo na frente mas com os macros
    desarmados (o "apertei e não veio nada");
  - um retrato do PC a cada sessão: Windows, monitor, layout de teclado, RAM,
    elevação, `LowLevelHooksTimeout`, vídeo e limite de FPS do jogo e quais
    programas conhecidos por mexer em teclado, FPS ou overlay estão rodando.

  O card mostra os números da sessão (disparos, hold e intervalo medidos, teclas
  recusadas, saúde do hook). **Exportar relatório** junta retrato, números, eventos e
  o fim do `app.log` (sem o nome do usuário do Windows) num JSON só, para o
  testador mandar; **Abrir pasta** abre a
  pasta de configuração. O registro não é keylogger: entram só as teclas que o
  próprio macro manda, e título de janela só do jogo ou do app (das outras, o nome
  do executável). O arquivo gira em 4 MB para `debug.old.jsonl`.
- **Painel de teclas no overlay** com o modo debug ligado: a última sequência acende
  tecla a tecla enquanto sai, com o hold real de cada direção embaixo (em vermelho
  quando fica abaixo de um quadro no limite de FPS do jogo). Pensado para gravar a
  tela e comparar, quadro a quadro, com as setas que o jogo acende. Fica na borda
  direita da tela, só com o jogo na frente e nunca sobre a tela cheia exclusiva.
- **Teste de digitação**: o app digita 10 sequências (Reforço e Rearmar Eagle, que
  tem ↑↑) na própria janela, com o perfil, o modo de setas e a tecla de menu
  escolhidos, e confere se cada tecla chegou e na ordem. Separa "o PC come as
  teclas" de "o jogo não as vê".

### Alterado

- **Logo e cores novos.** O ícone do app, da bandeja e do instalador passou a ser a
  caveira amarela, e os dois temas usam as cores dela: o claro é amarelo com preto
  de apoio e o escuro, preto com amarelo. O layout não mudou, e o tema já escolhido
  nas configurações continua valendo.
- **Rolar de novo não repete o item da rodada anterior** enquanto houver outra
  opção, em todas as categorias e nas duas sub-abas de sorteio. Na Meta, com só três
  armas no topo de cada categoria, a primeira colocada saía várias vezes seguidas.
  Item travado continua no lugar, e uma categoria com um item só repete.
- Salvar com o nome de uma build que já existe pergunta antes (**Substituir?**, por
  3s) em vez de sobrescrever em silêncio.
- Excluir uma build pede um segundo clique (**Confirmar?**, por 3s). O botão fica à
  vista, e não só sob o mouse.
- Depois de salvar, o campo de nome continua com o nome da build.
- O perfil **Padrão** agora diz **40 fps**: com o pior jitter o hold dele cai a
  29 ms, abaixo de um quadro de 30 fps. Os números não mudaram.
- Os cinco perfis de velocidade ficam em duas linhas, três por linha.

### Corrigido

- **Ícones dos boosters.** A wiki publica os ícones de booster e de passiva de
  armadura só em SVG, que o app não decodifica, e os cards mostravam o quadrado
  vazio. Os 48 ícones agora vêm em WebP da API de dados. As passivas do topo na
  sub-aba Meta também ganharam ícone.
- **Estratagema anunciado não entra mais na grade antes de sair no jogo.** A API de
  dados passou a listar o que a wiki já anunciou (o campo `upcoming`), e a
  sincronização acrescentava um anunciado que já tivesse setas e ícone como se ele
  estivesse no jogo. Agora ele espera o lançamento, como o equipamento já esperava.

### Desenvolvimento

- **`npm run scrape` lê da API de dados, não mais da wiki.** O novo
  `scripts/sync-equipment.mjs` monta o `equipment.json` inteiro e os ícones a partir
  da API em poucos segundos, sem o rate-limit da wiki e sem o passo de
  `optimize-images`. As regras de conversão são as mesmas que o app usa em runtime,
  então um item baixado entre releases entra no release seguinte com o mesmo id. Id
  de item existente nunca muda, e o script não grava nada se uma categoria perder
  mais de 10% dos itens. O `scripts/scrape-wiki.mjs` saiu.
- `equipment.json` regenerado pela API, com os itens novos listados acima. Os ícones
  de equipamento passam de 200px para até 256px, o tamanho que o app guarda, e as
  fichas de arma perdem os restos de marcação da wiki no dano. Todas as listas ficam
  em ordem alfabética (warbonds por data).
- `InputSink::send` devolve se o `SendInput` aceitou a tecla, e o aviso de recusa no
  `app.log` traz o erro do Windows.
- `npm run sync-stratagems` ignora estratagema anunciado e deixa de fora, com aviso,
  estratagema novo que a API ainda manda sem imagem (`image: null`), em vez de
  morrer no download do ícone.

## [2.0.0] - 2026-09-19

**Reescrita completa.** O app deixou de ser um Electron com React e virou um binário
nativo em **Rust sobre Win32 puro**: um processo, janela e overlay desenhados em
Direct2D, entrada por `SendInput` com scancode, atalhos por `WH_KEYBOARD_LL` e foco
por `SetWinEventHook`. Não há Chromium, Node, nut.js nem processo separado de
renderer. Toda a funcionalidade da v1 foi portada; as exceções estão em
**Removido**, e são deliberadas.

A versão traz também uma **interface nova** (visual neobrutal, com tema escuro e
claro) e a sincronização com a wiki da comunidade (helldivers.wiki.gg) até
09/09/2026, incluindo a warbond **Castellan's Creed** (Helldivers 2 × Warhammer
40.000, 12/08/2026) e a campanha **Void Piercer**, que nunca chegaram a sair numa
versão 1.x. Daqui em diante, estratagema novo e sequência trocada num patch chegam
sem precisar de versão nova do app.

> **Vindo da v1?** Baixe o instalador manualmente: o auto-update da v1 não instala a
> v2. As configurações migram sozinhas; slots e builds salvas vêm pelo backup JSON
> (veja **Migração da v1**, abaixo).

### Adicionado

- **Estratagemas novos sem atualizar o app.** Junto com a checagem de atualização
  (nunca com o jogo em foco), o app consulta a
  [API de dados](https://helldivers-api.dionatha.com.br) e baixa o estratagema que
  saiu depois da versão instalada, com o ícone. Ele aparece na próxima abertura do
  app, no fim do grupo dele (descartável com descartáveis, sentinela com
  sentinelas…); a posição exata vem no release seguinte. Tudo o que chega da API é
  validado antes de virar atalho.
- **Sequência trocada num patch chega sem atualizar o app.** Pela mesma consulta, o
  app adota o codex novo de um estratagema que já existe. Uma sequência que repita a
  de outro estratagema ou comece com a de outro é recusada, e mais de 5 trocas de uma
  vez são tratadas como dado quebrado (nenhuma vale).
- **Modo Meta sempre no patch mais novo.** O app descobre sozinho o patch atual do
  helldive.live, em vez de ficar preso no patch do release, e casa os slugs de
  estratagema que o `statsMap.json` embarcado não conhece, só quando o par é
  inequívoco.
- **Dois temas**: `rose` (escuro, o padrão) e `crimson` (claro). O app segue o modo
  claro/escuro do Windows até a primeira escolha manual: pelo toggle da topbar, por
  `Shift+T` ou pelo painel **Tema** em Configurações, que também volta a seguir o
  sistema. A escolha vai no `settings.json` (e no backup) como `theme`; o overlay
  troca junto.
- **Barra de título no tema** (Windows 11): cor da página, moldura e cantos retos.
- **Estratagema Eagle Gas Airstrike** (`⬆️➡️⬅️➡️`), o 92º de loadout.
- **Estratagema 40-K Meltagun** (`⬇️⬅️⬆️⬅️⬅️⬇️`), da Castellan's Creed, com ícone
  próprio.
- **Armadura KDM-500 Outrider** e a passiva **Kinetic Displacement Mitigation**.
- **Capacete IX-Voidwalker**, da campanha Void Piercer.
- **Warbonds no arsenal**: `equipment.json` traz as 24 warbonds com capa, tipo
  (Padrão / Premium / Lendária), data de lançamento e preço.
- **Tentar de novo** no erro das estatísticas da sub-aba Meta e no erro do updater.
- **Log de diagnóstico** em `app.log`, na pasta de configuração
  (`%APPDATA%\Macro Helldivers 2`). Sem console, é onde ficam as falhas que
  acontecem depois do boot.
- **Aviso de erro no boot**: se a instalação estiver sem a pasta `assets/`, o app
  explica em uma caixa de diálogo em vez de morrer em silêncio.
- **Licença de uso pessoal** (`LICENSE`, instalada junto como `LICENSE.txt`): uso
  pessoal e não comercial; publicar o app, original ou modificado, ou mostrá-lo em
  vídeos e posts exige crédito ao autor. Até aqui o repositório não tinha arquivo de
  licença; só o `Cargo.toml` declarava MIT.

### Alterado

#### Macro e atalhos

- **Motor de macro em thread própria** com prioridade elevada, `timeBeginPeriod(1)` e
  espera híbrida (`spin_sleep`): a sequência não depende mais do event loop do Node.
  As teclas saem por **scancode** em vez de virtual-key, que é o que o jogo lê.
- **Nenhuma tecla fica presa no jogo**: modificador e direções são soltos por RAII,
  então perder o foco no meio de uma sequência (ou qualquer erro) solta tudo. Um
  panic hook faz o mesmo antes de o processo abortar.
- **Foco por evento, não por polling**: `SetWinEventHook(EVENT_SYSTEM_FOREGROUND)`
  substituiu o timer que lia o título da janela ativa em intervalo fixo. Alt-tab
  desarma os atalhos na hora e aborta a sequência em andamento.
- **Perfis de velocidade rotulados pelo FPS que garantem** (`Padrão · 30 fps`,
  `Rápida · 60 fps`, `Turbo · 60+ fps`). Veja a correção em **Corrigido**.

#### Interface e overlay

- **Visual novo em todas as telas e no overlay**, seguindo o `styleguide.md` (skin
  `retro`, neobrutal): cantos retos, moldura de 2px, sombra dura deslocada, painéis
  com barra de título (`ATALHOS_DE_COMBATE.CFG`) e os quadrados de janela, clicáveis
  que sobem no hover, caret piscando (`ESCUTANDO_`, `VERIFICANDO_`) no lugar de
  pulsos, estados vazios (`> NADA AQUI`), aviso de tela cheia como banner e o
  resultado do backup como toast. Os tiles da grade mostram o ícone inteiro, com
  nome e codex numa legenda embaixo.
- **Fonte JetBrains Mono** (400, 700, 800 e 800 itálico), embutida no app.
- **A janela abre em 900x820**, o que mostra duas linhas inteiras da grade e a barra
  de slots sem rolar (a v1 abria em 820x640). O tamanho é travado na área útil do
  monitor, então numa tela baixa a janela nasce cabendo nela.
- **A grade ganha colunas em vez de esticar os tiles**: numa janela larga ou
  maximizada, a aba de macros e a grade da build personalizada acrescentam colunas e
  mantêm o tile perto de 184 DIP, que é o tamanho para o qual o ícone do jogo foi
  feito. Com quatro colunas fixas, uma janela maximizada levava o tile a mais de
  400 DIP e o ícone saía ampliado e borrado; os ícones agora ficam em memória no
  tamanho de origem (256px).
- **"Mostrar animações no Windows" desligado** congela os fades e o caret e esconde
  a textura de scanline.
- **Interface sem loop de render**: a janela e o overlay só repintam em mudança de
  estado (clique, evento recebido ou animação em curso). Parado, o app não desenha;
  minimizado ou na bandeja, nem as animações rodam.
- **Overlay dimensionado por estado**: a janela transparente continua sempre viva
  (escondê-la roubaria o foco do jogo), mas mede 1×1 quando não há nada a mostrar,
  o retângulo do strip no modo minimal e 840×660 no painel. Antes o compositor
  empilhava uma camada do tamanho do monitor sobre o jogo em todo frame.

#### Estratagemas, arsenal e ícones

- **Ordem dos estratagemas igual à do jogo.** A grade reproduz célula a célula os
  menus do destroyer: a Eagle Gas Airstrike entra entre a Cluster Bomb e a Napalm
  Airstrike, e a 40-K Meltagun fica logo depois da GL-21 Grenade Launcher.
- **Ícones dos estratagemas redesenhados**: a wiki migrou para SVG com paleta nova e
  moldura colorida por tipo de permissão (vermelho / ciano / verde), que agora bate
  com as cores das categorias na interface. Todos os 92 foram regerados.
- **23 renders de equipamento atualizados** para as versões novas da wiki: armaduras e
  capacetes A-35 Recon, A-9 Helljumper, CE-64 Grenadier, PH-9 Predator, TG-8
  Sharpshooter e TG-122 Demo-Trooper; as armas da colaboração 40-K (R/40-K Hot-Shot,
  P/40-K Bolt Pistol, G/40-K Melta Mine); e as capas Camo Cloak, City Fighter's
  Resolve, Cloak of Posterity's Gratitude, Dissident's Nightmare, Ingress-81, Mark of
  the Crimson Fang, Triangulation Veil e Tyrant Hunter.
- **Estratagemas renomeados** conforme o jogo: os drones perderam a alcunha "Guard Dog"
  (`AX/LAS-5 Rover`, `AX/ARC-3 K-9`, `AX/FLAM-75 Hot Dog`, `AX/TX-13 Dog Breath`,
  `AX/AR-23 Guard Dog`), `M-102 Fast Recon Vehicle` virou `M-102 Gunner FRV` e
  `SH-20 Ballistic Shield Supply` virou `SH-20 Ballistic Shield Backpack`. Os IDs não
  mudaram: slots e builds salvas continuam válidos.
- **Nomes das armas 40-K corrigidos** (`R/40-K Hot-Shot Marksman Rifle`,
  `P/40-K Bolt Pistol`, `G/40-K Melta Mine`) e a capa `United in Equality` perdeu o
  sufixo `(Cape)`.
- **Ícones em WebP**: a biblioteca inteira saiu de PNG para WebP com as mesmas
  dimensões (a conversão levou 429 imagens de 17,7 MB para 3,0 MB). O ícone da
  bandeja é um `tray.png` de 64 px em vez do `icon.png` de 1024 px.
- **Dados de equipamento carregados sob demanda**: `equipment.json` e `statsMap.json`
  só são lidos na primeira visita à aba Builds.

#### Instalação, atualização e arquivos

- **Instalador NSIS per-machine** no lugar do electron-builder, com desinstalação
  automática da v1 antes de instalar e preservação de `%APPDATA%\Macro Helldivers 2`.
- **Atualização não baixa mais sozinha durante a partida**: a checagem fica adiada
  enquanto o jogo está em foco e o download exige confirmação. O download tem prazo
  total (uma conexão travada não prende o updater pela sessão), e o "Depois" do modal
  mantém o "Instalar agora" no rodapé.
- **Configurações gravadas de forma atômica**, com fsync: queda de energia no meio do
  save não deixa um JSON vazio. Um arquivo ilegível é preservado como `.bad` em vez de
  ser sobrescrito pelo próximo save, e assim um bloqueio momentâneo de antivírus não
  custa as builds salvas.

### Removido

Decisões da reescrita, não regressões:

- **Opção "modificador de sprint"** (adicionada na 1.0.0). O hook de teclado de baixo
  nível dispara com qualquer modificador pressionado, então correr e chamar um
  estratagema já funciona sem configurar nada. A opção não tinha mais o que resolver.
- **Animação de abertura** (intro CRT de 4,5 s). O app abre direto na interface.
- **Configurações e criação de builds dentro do overlay.** O painel ficou com o que
  se faz de mouse no meio da partida: atribuir slots e aplicar builds salvas. O resto
  vive na janela principal, que é onde o teclado chega: o overlay não recebe teclado
  por design, e é isso que garante que ele nunca roube o foco do jogo.

### Corrigido

- **Perfis de velocidade que perdiam input.** O jogo lê o teclado uma vez por quadro
  (16,7 ms a 60 fps), e os perfis Rápida e Turbo seguravam a tecla por 15 ms e 10 ms;
  o jitter de ±5 ms chegava a derrubar o Turbo para 5 ms. Uma tecla que sobe e desce
  entre dois quadros não existe pro jogo, e o estratagema falhava de forma
  intermitente. Agora cada perfil tem um piso de tempo de tecla medido em quadros, e
  a velocidade vem do intervalo entre teclas.
- **Latência do disparo**: a sequência de teclas começa antes dos avisos para a
  interface, não depois.
- **Casamento de metadados da wiki**: nomes curtos podiam casar com o estratagema
  errado por substring, contaminando as regras de build balanceada.
- **Mapeamento de estatísticas quebrado pelos renames**: os slugs `guard_rover`,
  `guard_arc`, `guard_hot`, `guard_breath` e `backpack_ballistic` deixaram de casar
  com os nomes novos e sumiam do modo Meta. Corrigidos na tabela de apelidos.
- **Números trocados entre as emplacements no modo Meta**: a Grenadier Battlement
  aparecia com os da AT Emplacement, a HMG Emplacement com os da Grenadier, e a AT
  Emplacement sumia. O FRV original (`frv`) estava ligado ao Supply FRV em vez do
  Gunner FRV. O `statsMap.json` foi regenerado com o patch 13, que traz também
  Meltagun, Supply FRV, Bolt Pistol, Hot-Shot, Melta Mine e True Grit.
- **Builds salvas**: nome em branco não sobrescreve mais uma build existente ("Build 2"
  colidia depois de excluir a 1); o chip de build ativa acende também para builds
  antigas que precisaram de saneamento; passivas do top meta são validadas contra o
  equipamento atual.
- **Imagens órfãs no arsenal**: oito arquivos com `39` no nome (resíduo de `&#39;` mal
  decodificado em uma extração antiga) foram removidos.

Também foram corrigidos, antes do lançamento, bugs da própria reescrita que nunca
saíram numa versão: vazamento de brushes do Direct2D em cores animadas; overlay
congelado depois de uma perda de dispositivo da GPU; reentrância no `WndProc` a partir
dos campos de busca; um `EDIT` oculto que retinha o teclado depois de trocar de aba;
overlay que só se reposicionava segundos depois de uma mudança de resolução ou DPI;
ícone da bandeja que não voltava depois de o Explorer reiniciar; consultas de
estatísticas duplicadas ao alternar facção (e o cache perdendo entradas em escrita
concorrente); e um console preto aberto junto com o exe de release.

### Segurança

- **O updater verifica o SHA-256 do instalador** contra o `.sha256` publicado no
  release, e de novo na hora de executar, porque o exe espera em `%TEMP%` (gravável
  por qualquer processo do usuário) e roda com o token elevado do app.

### Migração da v1

- **O auto-update da v1 não chega na v2**: o electron-updater procura um
  `latest.yml` que o release novo não publica. A atualização é pelo instalador
  baixado da página de releases.
- O instalador da v2 **desinstala a v1** antes de instalar e preserva a pasta
  `%APPDATA%\Macro Helldivers 2`.
- **Configurações são migradas** na primeira execução: atalhos dos slots e dos apoios,
  tecla do menu de estratagemas, modo setas, velocidade, idioma, overlay e HUD
  persistente. A opção de modificador de sprint é descartada.
- **Slots e builds salvas não migram sozinhos**: viviam no armazenamento interno do
  Chromium, que não existe mais. O caminho é exportar o backup JSON na v1
  (**Configurações → Backup → Exportar**) e importá-lo na v2; o formato do arquivo é
  o mesmo.

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

### Desenvolvimento

Nada aqui muda o app instalado:

- **Crate Rust** com `rust-toolchain.toml` pinando canal, componentes e o alvo
  `x86_64-pc-windows-msvc`. Todo código win32 fica atrás de `#[cfg(windows)]`, com
  stubs que deixam `cargo check` e `cargo test` rodarem no macOS e no Linux.
- **CI** (`ci.yml`) em `windows-latest`: `fmt`, `clippy -D warnings`, `check`, testes
  no alvo MSVC e build de release a cada push e PR.
- **Release por tag** (`release.yml`): uma tag `vX.Y.Z` roda os testes, compila,
  gera o instalador NSIS e o `.sha256` e publica o release, com as notas tiradas
  desta seção do changelog.
- **Bancadas** `timing_bench` (desvio do relógio do motor) e `soak` (1.000 execuções
  in-game).
- **Pipeline de dados** em `scripts/` (Node, fora do app):
  - `npm run sync-stratagems` lê a [API de dados](https://helldivers-api.dionatha.com.br),
    que já entrega a wiki em JSON com os ícones em WebP, em vez de raspar a wiki.
    Casa as entradas pelo `slug` (campo novo em `stratagems.json`), nunca reordena o
    que existe e põe estratagema novo no fim do subgrupo, com `id` derivado do slug:
    o mesmo que o app dá em runtime. Aborta sem escrever se algum codex repetir ou
    começar com outro.
  - `sharp` substituiu o ImageMagick e o `cwebp`: nenhum binário externo é
    necessário.
  - `npm run stats-map` junta os slugs dos dois patches mais novos do helldive.live,
    para não perder o par de um item que ninguém usou no último.
  - O cache de download nunca serve SVG velho, e o `optimize-images` reescreve os
    ícones dos apoios fixos em `src/data.rs`.
- **App Electron removido** depois da validação de paridade. A v1 continua acessível
  pela tag [`v1.0.0`](https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v1.0.0).

## [1.0.0] - 2026-08-08 (pré-release)

Primeira versão 1.x, e a última sobre Electron.

### Adicionado

- **Perfis de velocidade** (Padrão, Rápida e Turbo) com intervalos humanizados: um
  jitter aleatório de ±5 ms em cada espera.
- **Overlay in-game não-ativável** (`Ctrl + H`): a janela transparente nunca rouba o
  foco do jogo, com strip de slots opcional sempre visível e aviso quando o jogo está
  em "Tela Cheia" (modo incompatível com overlay de janela).
- **Central de Builds** com três modos:
  - **Meta**: pick rates reais da comunidade (helldive.live) por facção e dificuldade,
    com sorteio ponderado pelo top exibido.
  - **Aleatória**: sorteio completo com regras de sets de armadura, loadout balanceado
    (1 arma de apoio + 1 mochila) e máximo de 1 torreta.
  - **Personalizada**: montagem manual do loadout: escolha do slot, grade de
    estratagemas com busca, equipamento opcional e importação dos slots atuais do macro.
- **Builds salvas**: nomeie qualquer build e aplique nos slots de macro com um clique,
  inclusive pelo overlay.
- **Arsenal completo**: base de dados de armas, armaduras, capacetes, capas, boosters e
  passivas extraída da wiki da comunidade, com ícones offline.
- **Busca de estratagemas** sem acento e sem diferenciar maiúsculas.
- **Backup**: exportação e importação de builds, slots e configurações em JSON.
- **Liberar um slot**: botão de remover no hover, e clicar no estratagema que já está
  no slot ativo o desequipa.
- **Feedback de execução**: cards de apoio e slots acendem quando o macro dispara e
  ficam vermelhos quando o disparo é recusado por outro macro em andamento; o rodapé
  mostra se o jogo está sendo detectado.
- **Modificador de sprint configurável**: quem corre com Alt ou Ctrl escolhe a tecla
  (antes era sempre Shift).
- **Posição e tamanho da janela lembrados** entre sessões, descartando posições fora
  de qualquer monitor conectado.
- **Estratagemas M-103 Supply FRV e M-104 Incinerator FRV**, com a tag de
  exclusividade `Vehicle`: só um veículo por loadout, como já valia para exotrajes.

### Alterado

- **Boot mais rápido**: o polling de foco (que carrega os módulos nativos do `nut.js` e
  bloqueia o processo principal por cerca de 1s) e a criação da janela de overlay agora
  acontecem depois da primeira pintura da janela principal, eliminando o travamento na
  abertura do app.
- **Veículos reordenados** na categoria de suprimento (TD-220, M-104, M-103, M-102,
  depois os exotrajes) e **códigos dos exotrajes** EXO-55 Breakthrough e EXO-51
  Lumberer atualizados.
- **Um macro por vez**: disparos simultâneos são recusados com aviso, em vez de
  descartados em silêncio.

### Corrigido

- **Estratagemas falhando no meio da sequência** em máquinas mais lentas: o atraso
  automático do `nut.js` subiu de 1 ms para 10 ms.
- **Ícone da bandeja ausente no app instalado**: o `extraResources` copiava o ícone para
  `resources/public/icon.png` enquanto o processo principal procurava em
  `resources/icon.png`. Sem ícone, a criação do `Tray` falhava e a janela escondida ao
  fechar ficava inalcançável. O ícone agora é copiado para o caminho esperado, com
  fallbacks na resolução, e fechar só esconde a janela quando existe bandeja.

## [0.3.0] - 2026-04-28 (instável)

### Adicionado

- **Overlay in-game** (`Ctrl + H`), com modo minimalista para ver e configurar os
  macros por cima do jogo.
- **Exotrajes EXO-55 Breakthrough e EXO-51 Lumberer**, com ícones.
- **Instância única**: abrir o app de novo traz a janela existente para a frente.
- **Animação de abertura**, com as frases traduzidas.
- **Versão do app** exibida no rodapé.

### Alterado

- **IDs dos estratagemas reindexados** em ordem crescente; os exotrajes ocupam os IDs
  66 a 69.
- **Atalhos registrados só com o jogo em foco**, e detecção da janela do jogo mais
  robusta a variações do título.
- **Electron atualizado**, fechando vulnerabilidades conhecidas das dependências.

### Corrigido

- Ícones dos estratagemas de apoio que não carregavam no app instalado.
- Ponte do auto-update entre o processo principal e a interface.

## [0.2.0] - 2026-04-25

### Adicionado

- **Idiomas**: português e inglês.
- **HUD tático**: visual novo com cores por categoria (vermelho ofensivo, verde
  defensivo) para reconhecer os estratagemas de relance.

### Alterado

- **Ícones em WebP**, redimensionados para 256 px.
- Transições e animações com aceleração por GPU.

## [0.1.0] - 2026-04-25

Primeira versão funcional.

### Adicionado

- **4 slots de macro** configuráveis, executados por emulação de teclado (`nut.js`)
  com o app elevado.
- **Tecla do menu de estratagemas** à escolha: Ctrl, Alt, `=` ou `-`.
- **Controle de foco**: os macros só disparam com a janela do jogo ativa.
- **Instalador NSIS** para Windows.

[2.1.0-beta.1]: https://github.com/DionathaGoulart/Macro-Helldivers2/compare/v2.0.0...v2.1.0-beta.1
[2.0.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/compare/v1.0.0...v2.0.0
[1.0.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/compare/v0.3.0-(unstable)...v1.0.0
[0.3.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/compare/v0.2.0...v0.3.0-(unstable)
[0.2.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/DionathaGoulart/Macro-Helldivers2/releases/tag/v0.1.0
