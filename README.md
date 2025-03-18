# ai-cli

A minimal command-line interface for interacting with the 1min.ai API.

## Installation

```bash
cargo install --path .
```

## Configuration

Before using the tool, you need to configure your 1min.ai API key:

```bash
ai-cli config
```

This will prompt you to enter your API key, which will be securely stored using the system's keyring.

## Usage

### Basic Usage

```bash
ai-cli "What is the capital of France?"
```

### Interactive Mode

```bash
ai-cli -i
```

Or start with an initial prompt:

```bash
ai-cli -i "Let's talk about programming languages"
```

### Image Generation Mode

Generate an image based on the prompt:

```bash
ai-cli -g "a cute anime cat"
```

### Voice Output

Enable voice output (uses the system's 'say' command):

```bash
ai-cli -v "Tell me a joke"
```

### Quiet Mode

Only output the response via voice (requires voice output to be enabled):

```bash
ai-cli -q -v "What's the weather like today?"
```

### Selecting a Model

Choose a specific AI model:

```bash
ai-cli -m gpt-4o "Explain quantum computing"
```

Available models include:
- o1-preview
- o3-mini (default)
- gpt-4o
- gpt-4o-mini
- deepseek-r1

## Options

- `-i, --interactive`: Enable interactive mode
- `-v, --voice-output`: Enable voice output of AI responses
- `-q, --quiet`: Do not print AI responses (only works with voice output)
- `-m, --model <MODEL>`: The AI model to use (default: "o3-mini")
- `-w, --words <WORDS>`: Maximum number of words for web search (default: 500)
- `-g, --image-generation`: Enable image generation mode (incompatible with interactive and voice modes)
- `-s, --size <SIZE>`: Image size (1024x1024, 1024x1792, 1792x1024) [default: 1024x1024]
- `-h, --help`: Print help
- `-V, --version`: Print version 
- `--quality <QUALITY>`: Image quality (standard, hd) [default: standard]
- `--style <STYLE>`: Image style (vivid, natural) [default: vivid]