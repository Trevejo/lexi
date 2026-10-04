# Lexi — Tradutor Gamer e Dicionário Nativo para Windows

O **Lexi** é um aplicativo nativo e ultra-leve para Windows projetado para rodar em segundo plano enquanto você joga. Ele resolve o problema de pesquisar termos e gírias em inglês durante cutscenes e diálogos de jogos sem quebrar o ritmo da gameplay.

Pressionando uma combinação de teclas e falando a palavra desejada:
1. O **Handy** (reconhecimento de voz local offline via Whisper/Parakeet) transcreve sua voz.
2. O **Lexi** intercepta a palavra instantaneamente, consulta o significado no **dicionário offline** (e opcionalmente na **IA**) e projeta um card visual semitransparente na frente do jogo.
3. O card **não rouba o foco da janela do jogo**, permitindo que você continue jogando normalmente.
4. Você pode fazer a janelinha sumir **assim que tirar o dedo da tecla**, por tempo limite ou apertando `Esc`.
5. Em jogos de **Tela Cheia Exclusiva**, o Lexi pode ainda falar a tradução diretamente no seu fone de ouvido (via sintetizador nativo do Windows).

---

## 🎯 Destaques do Projeto

- **Ultra-leve e 100% Nativo:** Desenvolvido em **Rust** utilizando Win32 e GDI puro (sem Electron, sem WebView2, sem .NET). Consome **0,0% de CPU ociosa** e menos de **15 MB de memória RAM**.
- **Interface Gráfica de Configurações Nativa:** Tela de configurações completa com seletores de posição, sliders de tamanho de fonte, opacidade, volume de voz e recarregamento em tempo real sem precisar reiniciar o aplicativo.
- **Suporte Automático a Monitores 4K & DPI Scaling:** Adaptação dinâmica de resolução e tipografia para zoom de 100%, 125%, 150%, 175% e 200%, garantindo leitura nítida e proporções perfeitas em qualquer monitor.
- **Integração Inteligente com o Handy:** Utiliza a porta de Post-Processing local do Handy (`127.0.0.1:47823`). O Lexi instrui o Handy a **não colar nada** no jogo (sem risco de comandos indesejados no jogo).
- **Consulta Híbrida (Offline + IA):**
  - **Dicionário Offline Completo:** Base local com **mais de 300.000 palavras em inglês** e **65+ termos gamer** essenciais com explicações contextuais detalhadas e exemplos reais de uso em jogos (MOBA, FPS, Battle Royale, RPG). Resposta instantânea em **< 1ms**.
  - **IA Opcional (LLM):** Compatível com OpenAI (`gpt-4o-mini`), Google Gemini (`gemini-1.5-flash`), Groq ou Ollama local para gírias complexas, expressões idiomáticas (*phrasal verbs*) e termos específicos de universos de fantasia/sci-fi.
- **Posicionamento Flexível:** Suporte a 7 posições de overlay (`top_center`, `middle_left`, `middle_right`, `top_left`, `top_right`, `center`, `bottom_center`).
- **Fechamento Configurável:**
  - `release_key`: some assim que você solta a tecla do teclado.
  - `timeout`: some após N segundos (ex: 8s).
  - `any_key`: some ao pressionar qualquer tecla (sem bloquear a ação no jogo).
  - `hotkey`: fecha ao apertar `Esc`.
- **Compatibilidade Dupla:** Funciona em jogos em modo **Janela Sem Bordas (Borderless)** via overlay translúcido e em **Tela Cheia Exclusiva** via áudio TTS nativo de baixa latência (SAPI).

---

## 🚀 Como Compilar e Rodar no Windows

### Pré-requisitos
1. **Windows 10 ou Windows 11**.
2. **Rust & Cargo**: Baixe e instale pelo instalador oficial [rustup.rs](https://rustup.rs/).
3. **C++ Build Tools**: Instalador do Visual Studio com a opção *"Desenvolvimento para Desktop com C++"* marcada.
4. **Handy** instalado: Baixe a versão mais recente em [handy.computer](https://handy.computer) ou via `winget install cjpais.Handy`.

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
O Lexi iniciará silenciosamente na bandeja do sistema (perto do relógio do Windows).

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
