# 🛡️ Macro Helldivers 2

Uma ferramenta de alto desempenho, minimalista e segura para automatizar o acionamento de Estratagemas no Helldivers 2.

![Screenshot](public/stratagems/Orbital_Precision_Strike_Stratagem_Icon.png)

## 🚀 Funcionalidades

- **Zero Delay**: Sistema de cache de janela ativa para resposta instantânea.
- **Segurança Antidetecção**: Emulação de hardware via `nut.js`.
- **Inteligência de Janela**: O macro só funciona quando o Helldivers 2 é a janela em foco.
- **Sempre Ativo**: Roda silenciosamente na bandeja do sistema.
- **Personalização**: Escolha entre WASD ou Setas para os códigos.
- **Suporte a Sprint**: Funciona mesmo enquanto você corre (Shift segurado).

## 📥 Como Instalar e Usar

1. Baixe o instalador `.exe` na aba **[Releases](https://github.com/DionathaGoulart/Macro-Helldivers2/releases)**.
2. Execute o instalador e abra o app.
3. Configure seus 4 slots de estratagemas favoritos.
4. No jogo, certifique-se de que a tecla de abrir o menu de estratagemas é a mesma selecionada no app (Ctrl, Alt, = ou -).
5. **Recomendado**: Use as Setas do teclado no app para não interferir no seu movimento WASD.

---

## 🔄 Como funcionam as Atualizações Automáticas?

O app possui um sistema integrado de **Auto-Update**. 

### Como o app se atualiza:
1. Sempre que você abre o app, ele consulta o repositório no GitHub para ver se existe uma versão mais recente que a sua.
2. Se houver, ele baixa a atualização em segundo plano.
3. Na próxima vez que você abrir o app, ele instalará a nova versão automaticamente.

---

## 🛠️ Desenvolvimento e Compilação

Se você deseja modificar o código ou buildar manualmente:

### Pré-requisitos:
- Node.js v18+
- Modo de Desenvolvedor ativado no Windows (para links simbólicos).

### Comandos:
```bash
# Instalar dependências
npm install

# Rodar em modo desenvolvedor
npm run dev

# Gerar instalador .exe
npm run build
```

---

## 🏗️ Estrutura do Projeto (Padrão Industrial)

```text
├── src/
│   ├── main/          # Processo Principal (Electron)
│   ├── renderer/      # Interface (React + Vite)
│   ├── macro/         # Motor de Execução (nut.js)
│   └── preload/       # Ponte de Segurança IPC
├── public/            # Assets estáticos e ícones
└── package.json       # Configurações de Build e Dependências
```

---
Criado por **DionathaGoulart**. Liberdade ou Morte! ⬆️➡️⬇️⬇️⬇️
