# Pocket-Tray

<div align="center">
  <img width="256" height="256" alt="pipertray_icon_large" src="https://github.com/user-attachments/assets/d603300e-f7d6-4a83-9cf4-70be48ad6194" />
</div>

A standalone Windows system tray application for offline text-to-speech. Monitors your clipboard and automatically speaks copied text using the Pocket TTS neural speech synthesis engine.

![Pocket-Tray Icon](https://img.shields.io/badge/Platform-Windows%2010-blue) ![Rust](https://img.shields.io/badge/Language-Rust-orange) ![Offline](https://img.shields.io/badge/Mode-Offline-green)

## Features

- **Clipboard Monitoring** - Automatically speaks text when you copy it
- **8 Voice Options** - Choose from alba, azelma, cosette, eponine, fantine, javert, jean, or marius
- **Animated Tray Icon** - Visual feedback with animated equalizer bars while speaking
- **Completely Offline** - No internet connection required
- **Settings Persistence** - Remembers your voice selection and monitoring state
- **Single Executable** - Just one `.exe` file plus the models folder

## Installation

1. Download the latest [release](https://github.com/jame25/pocket-tray/releases).
2. Download the models / voice files from [here](https://huggingface.co/kyutai/pocket-tts).
3. Extract both to your preferred location.
4. Ensure the folder structure looks like this:

```
pocket-tray/
├── pocket-tray.exe
└── models/
    ├── tts_b6369a24.safetensors   (225 MB - main model)
    ├── tokenizer.model             (58 KB)
    ├── alba.safetensors
    ├── azelma.safetensors
    ├── cosette.safetensors
    ├── eponine.safetensors
    ├── fantine.safetensors
    ├── javert.safetensors
    ├── jean.safetensors
    └── marius.safetensors
```

4. Run `pocket-tray.exe`

## Usage

### Tray Menu Options

Right-click the tray icon to access:

| Option | Description |
|--------|-------------|
| **Monitoring** | Toggle clipboard monitoring on/off (enabled by default) |
| **Stop** | Stop current speech playback |
| **Voices** | Submenu to select from 8 available voices |
| **Quit** | Exit the application |

### How It Works

1. Launch the application - it appears in your system tray
2. With "Monitoring" enabled, copy any text to your clipboard
3. The text will be spoken automatically
4. The tray icon animates while speaking
5. Use "Stop" to interrupt speech, or copy new text (new text is ignored while speaking)

### Settings

Settings are automatically saved to `pocket-tray.json` next to the executable:

```json
{
  "monitor_enabled": true,
  "current_voice": "alba"
}
```

## Building from Source

### Prerequisites

- Rust toolchain (1.75+)
- Windows 10 SDK (for Windows builds)

### Build Commands

```bash
# Development build
cargo build -p pocket-tray

# Release build (optimized)
cargo build --release -p pocket-tray

# Cross-compile for Windows from Linux (requires appropriate toolchain)
cargo build --release -p pocket-tray --target x86_64-pc-windows-msvc
```

The executable will be in `target/release/pocket-tray.exe`

## Technical Details

### Architecture

- **TTS Engine**: Pocket TTS (FlowLM + Mimi neural codec)
- **Audio**: 24kHz sample rate, streaming playback via rodio
- **GUI**: Native Windows system tray via tray-icon + muda
- **Threading**:
  - Main thread: Event loop and UI
  - TTS thread: Model inference and audio generation
  - Clipboard thread: Polling for new text (500ms interval)

### Model Information

| Component | Size | Description |
|-----------|------|-------------|
| TTS Model | 225 MB | Main neural network weights |
| Tokenizer | 58 KB | SentencePiece tokenizer |
| Voice Embeddings | ~500 KB each | Pre-computed speaker embeddings |

### Performance

- Model load time: ~5-10 seconds (first launch)
- Generation: Real-time streaming (audio plays as it generates)
- Memory usage: ~500 MB during inference

## Credits

This project builds upon the excellent work of:

### Pocket TTS - Rust Implementation
**[babybirdprd/pocket-tts](https://github.com/babybirdprd/pocket-tts/tree/main/candle)**

The Rust port of Pocket TTS using the Candle ML framework, which this tray application is built upon.

### Pocket TTS - Original
**[kyutai-labs/pocket-tts](https://github.com/kyutai-labs/pocket-tts)**

The original Pocket TTS project by Kyutai Labs - a streaming text-to-speech model with voice cloning capabilities.

## License

This project follows the licensing terms of the upstream Pocket TTS projects. Please refer to:
- [Kyutai Labs Pocket TTS License](https://github.com/kyutai-labs/pocket-tts/blob/main/LICENSE)
- [Rust Port License](https://github.com/babybirdprd/pocket-tts/blob/main/LICENSE)

## Troubleshooting

### "Models directory not found"
Ensure the `models/` folder is in the same directory as `pocket-tray.exe` and contains all required files.

### No sound output
- Check your default audio output device
- Ensure the TTS model finished loading (tooltip shows "Ready")
- Try a different voice

### High CPU usage
The TTS model runs on CPU. This is normal during speech generation. CPU usage returns to minimal when idle.

### Application doesn't start
- Ensure you're running Windows 10 or later
- Check that Visual C++ Redistributable is installed
- Try running as administrator

## Contributing

Contributions are welcome! Please feel free to submit issues and pull requests.
