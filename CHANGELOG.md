# Changelog

Todas as mudanças relevantes deste projeto são documentadas aqui.

O formato segue [Keep a Changelog](https://keepachangelog.com/pt-BR/1.1.0/)
e o versionamento segue [Semantic Versioning](https://semver.org/lang/pt-BR/).

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
