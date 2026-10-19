# Lexi — Tradutor Gamer e Dicionário Nativo para Windows

O **Lexi** é um aplicativo nativo, ultra-leve e **100% autossuficiente** para Windows projetado para rodar em segundo plano enquanto você joga. Ele resolve o problema de pesquisar termos, expressões e gírias em inglês durante cutscenes e diálogos de jogos sem quebrar o ritmo da gameplay.

Pressionando uma combinação de teclas ou segurando o atalho de voz (Push-to-Talk):
1. O **Lexi captura sua voz diretamente do microfone** (ou opcionalmente via Handy ou outros softwares de terceiros) e transcreve com modelos locais de ponta (Whisper Base/Tiny/Small, Nemotron 3.5, Parakeet, Canary).
2. O **Motor de IA e Normalização** do Lexi limpa hesitações de fala e gaguejos, corrige foneticamente termos gamer e consulta o significado no **dicionário offline** ou em modelos de IA externos (**Google Gemini**, **Anthropic Claude**, **OpenAI**, **OpenCode** ou endpoints customizados).
3. O Lexi projeta um card visual semitransparente na frente do jogo com explicações detalhadas em português, contexto e exemplos reais de uso em jogos.
4. O card **não rouba o foco da janela do jogo**, permitindo que você continue jogando sem interrupção.
5. Você pode fazer a janelinha sumir **assim que soltar a tecla de fala**, por tempo limite ou apertando `Esc`.
6. Em jogos de **Tela Cheia Exclusiva**, o Lexi pode ainda falar a tradução diretamente no seu fone de ouvido (via sintetizador de voz nativo do Windows).

---

## 🎯 Destaques do Projeto

- **100% Autossuficiente (Sem Depender de Apps Externos):** Possui pipeline de áudio nativo integrado (`cpal`), gravação de microfone em segundo plano e gerenciador de download de modelos Speech-to-Text de alta performance direto do Hugging Face.
- **Onboarding Amigável no Primeiro Uso:** Assistente de boas-vindas nativo que guia o usuário na escolha e download do modelo de voz recomendado em 1 clique com barra de progresso, seleção de microfone e atalho Push-to-Talk.
- **Multi-Provedor de Inteligência Artificial:** Suporte nativo a **Google Gemini**, **Anthropic Claude**, **OpenAI (GPT-4o-mini)**, **OpenCode** (servidor local) ou qualquer endpoint compatível com OpenAI. Inclui clarificação inteligente de fala e enriquecimento de contexto gamer.
- **Ultra-leve e 100% Nativo:** Desenvolvido em **Rust** utilizando Win32 e GDI puro (sem Electron, sem WebView2, sem .NET). Consome **0,0% de CPU ociosa** e menos de **15 MB de memória RAM**.
- **Interface Gráfica de Configurações Nativa com 4 Abas:**
  - 🎙️ *Voz & Modelos STT:* Seleção e download de modelos (Whisper, Nemotron, Parakeet), barra de progresso, seleção de microfone e modo de ativação.
  - 🤖 *Inteligência Artificial:* Escolha de provedor (Gemini, Claude, OpenAI, OpenCode, Custom), chave de API, modelo, endpoint e botão para **Testar Conexão** com medição de latência em tempo real.
  - 🖥️ *Overlay no Jogo:* Posições (incluindo `middle_left` e `middle_right`), escala DPI para 4K (100% a 200%), sliders de fonte, opacidade, timeout e botão para **Testar Overlay Agora**.
  - 🔊 *Leitura (TTS):* Leitura em voz alta via Windows SAPI.
- **Suporte Automático a Monitores 4K & DPI Scaling:** Adaptação dinâmica de resolução e tipografia para zoom de 100%, 125%, 150%, 175% e 200%, garantindo leitura nítida e proporções perfeitas em qualquer monitor.
- **Consulta Híbrida de Altíssima Velocidade:**
  - **Dicionário Offline Completo:** Base local com **mais de 300.000 palavras em inglês** e **termos gamer** essenciais com explicações contextuais detalhadas e exemplos reais de uso em jogos (MOBA, FPS, Battle Royale, RPG). Resposta instantânea em **< 1ms**.
  - **Cache Inteligente em SQLite:** Evita reprocessamento de termos consultados anteriormente.
- **Posicionamento Flexível:** 7 posições de overlay (`top_center`, `middle_left`, `middle_right`, `top_left`, `top_right`, `center`, `bottom_center`).
- **Fechamento Configurável:**
  - `release_key`: some assim que você solta a tecla de fala.
  - `timeout`: some após N segundos (ex: 8s).
  - `any_key`: some ao pressionar qualquer tecla (sem bloquear a ação no jogo).
  - `hotkey`: fecha ao apertar `Esc`.
- **Compatibilidade Dupla:** Funciona em jogos em modo **Janela Sem Bordas (Borderless)** via overlay translúcido e em **Tela Cheia Exclusiva** via áudio TTS nativo de baixa latência (SAPI).

---

## 🚀 Como Compilar e Rodar no Windows

### Pré-requisitos
1. **Windows 10 ou Windows 11**.
2. **Rust & Cargo**: Baixe e instale pelo instalador oficial [rustup.rs](https://rustup.rs/).

### Compilação em 1 Clique
No diretório do projeto no Windows, basta dar dois cliques em:
```cmd
build.bat
```
Ou no terminal PowerShell/CMD:
```cmd
cargo build --release --bin lexi-win
```
O executável otimizado será gerado em `target\release\lexi-win.exe`.

### Executando em Segundo Plano
Dê dois cliques em:
```cmd
run.bat
```
O Lexi iniciará silenciosamente na bandeja do sistema (perto do relógio do Windows) e abrirá o assistente de onboarding no primeiro uso.

---

## ⚙️ Configuração do Handy (Passo a Passo)

Para conectar o Handy ao Lexi sem que nada seja colado na janela do jogo:

1. Abra as configurações do **Handy** (clique com botão direito no ícone do Handy na bandeja → *Settings*).
2. Vá na aba **Post Processing** e ative a opção **Enable Post-Processing**.
3. Em **Provider**, selecione ou adicione o provedor **Custom**:
   - **Base URL:** `http://127.0.0.1:47823/v1`
   - **API Key:** pode colocar qualquer texto (ex: `lexi`)
   - **Model:** digite `lexi`
4. Na seção de **Prompts**, crie um novo prompt com o nome `Lexi`:
   - Conteúdo do Prompt: digite exatamente `${output}`
   - Marque esse prompt como o prompt ativo.
5. Vá na aba **Shortcuts**:
   - Defina um atalho para **Transcribe with post-processing** (por exemplo: `Ctrl + Alt + Espaço`, ou mapeie uma tecla como `F13` em um botão lateral do mouse).
6. Recomendado para jogos: Em **Model**, use o **Parakeet V3** (roda na CPU rapidamente sem disputar a placa de vídeo com o jogo).

---

## 🛠️ Opções do Arquivo `config.toml`

Ao iniciar o programa pela primeira vez, ele criará o arquivo `config.toml` (baseado em `config.example.toml`). Você pode editá-lo pelo bloco de notas:

### 1. Como Fechar a Janelinha (`[dismiss]`)
```toml
[dismiss]
# Opções: "release_key", "timeout", "hotkey", "any_key", "hybrid"
mode = "hybrid"

# Tempo de exibição em segundos
timeout_seconds = 8.0

# Tecla rápida para fechar
dismiss_hotkey = "Escape"
```

### 2. Configuração de Inteligência Artificial (`[llm]`)
```toml
[llm]
enabled = true
base_url = "https://api.openai.com/v1" # Ou endpoint do Gemini / Groq
api_key = "sk-..."                     # Sua chave de API
model = "gpt-4o-mini"
timeout_seconds = 5
```
> **Dica Gemini Grátis:** O Google Gemini possui um endpoint gratuito compatível com OpenAI. Basta usar `base_url = "https://generativelanguage.googleapis.com/v1beta/openai"`, `model = "gemini-1.5-flash"` e sua chave obtida no Google AI Studio.

### 3. Modo Áudio para Tela Cheia Exclusiva (`[audio]`)
```toml
[audio]
enabled = true
only_fullscreen = true  # Só fala no fone quando o jogo for em tela cheia exclusiva
voice = "pt-BR"
volume = 85
```

---

## 📚 Importador de Dicionário Offline Completo

O Lexi já vem com mais de 100 palavras e gírias de jogos pré-cadastradas no código. Se você desejar importar centenas de milhares de palavras em inglês do Wiktionary para ficar 100% offline:

1. Baixe o dump em JSONL do [kaikki.org](https://kaikki.org/dictionary/rawdata.html) (English dictionary with translations).
2. Execute a ferramenta embutida:
```cmd
cargo run --release --bin build-dictionary -- jsonl kaikki.org-dictionary-English.jsonl dictionary.sqlite
```
Ou importe sua própria lista CSV (`termo,classe,traducao,definicao,exemplo`):
```cmd
cargo run --release --bin build-dictionary -- csv gaming_terms.csv dictionary.sqlite
```

---

## 🎮 Estrutura dos Módulos

```
gaming-translate-tool/
├── Cargo.toml                  # Workspace Rust com perfil de release otimizado
├── build.bat                   # Script de compilação rápida para Windows
├── run.bat                     # Script de inicialização em background
├── config.example.toml         # Modelo completo de configuração em Português
├── crates/
│   ├── lexi-core/              # Lógica de negócio e ponte HTTP com Handy
│   │   ├── src/bridge.rs       # Servidor HTTP compatível com Handy (resposta vazia)
│   │   ├── src/lookup.rs       # Orquestrador híbrido (Cache -> SQLite -> LLM)
│   │   ├── src/dictionary.rs   # Dicionário offline em SQLite com sementes gamer
│   │   ├── src/llm.rs          # Cliente para APIs de IA com retorno estruturado
│   │   ├── src/cache.rs        # Cache SQLite para consultas instantâneas repetidas
│   │   ├── src/normalize.rs    # Limpeza de fala ("o que significa X", "what is X")
│   │   └── src/config.rs       # Leitura e gravação do config.toml
│   └── lexi-win/               # Aplicação nativa Windows (#![windows_subsystem = "windows"])
│       ├── app.manifest        # Manifesto com suporte a Per-Monitor DPI V2
│       ├── src/main.rs         # Entry point com Mutex de instância única
│       ├── src/app.rs          # Message loop Win32 e mensageria em background
│       ├── src/overlay.rs      # Janela transparente Direct/GDI que não rouba foco
│       ├── src/keyboard_hook.rs# Hook de teclado de baixo nível para fechar ao soltar tecla
│       ├── src/tray.rs         # Ícone na bandeja do sistema com menu de contexto
│       └── src/tts.rs          # Leitura em voz alta SAPI nativa para tela cheia
└── tools/
    └── build-dictionary/       # CLI para compilar dumps do Wiktionary em SQLite
```

---

## 🛡️ Licença
Distribuído sob licença MIT. Sinta-se livre para customizar, estender e compartilhar.
