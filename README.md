# desk-talk

Transcription for your desktop.

A modern GUI application that records what you say when you press a button down, and types what you said when you release it.

> [!IMPORTANT]
> ⚠️ This video contains sound and is intended to be listened to with audio on. ⚠️

https://github.com/sloganking/desk-talk/assets/16965931/e5da605b-3a9d-4394-b4ec-a3de65605a65

## Features

✨ **Push-to-Talk Transcription** - Hold a key, speak, release to paste  
🎯 **System Tray Integration** - Runs quietly in the background  
⚙️ **Modern GUI Settings** - Beautiful, easy-to-use configuration interface  
📊 **WPM Statistics** - Track your words per minute with rolling averages  
🔒 **Secure API Storage** - API keys stored safely in Windows Credential Manager  
🎤 **Multiple Audio Devices** - Choose any microphone on your system  
🌐 **OpenAI or Local** - Use OpenAI's Whisper API or run models locally  
💾 **Persistent Settings** - All preferences saved between sessions

## Setup

Make sure [ffmpeg](https://www.ffmpeg.org/) is installed and added to your PATH

## Quickstart

1. **Download and run** the latest release
2. **Click the tray icon** in your system tray to open settings
3. **Configure your settings:**
   - Set your push-to-talk key (e.g., Scroll Lock)
   - Choose your microphone
   - Enter your OpenAI API key OR enable local transcription
4. **Click "Start Transcription"**
5. **Hold your PTT key, speak, and release!**

> [!NOTE]
>
> You can manage your OpenAI API keys at https://platform.openai.com/api-keys

## Using the GUI

### System Tray

- **Left-click** the tray icon to open the settings window
- **Right-click** for quick Start/Stop/Quit options

### Settings Tabs

**General Tab:**

- Configure push-to-talk key
- Select audio input device
- Toggle capitalization and spacing options
- Choose between paste mode (default) or typing mode

**Transcription Tab:**

- **OpenAI Mode:** Enter your API key for cloud transcription
- **Local Mode:** Download and run Whisper models on your computer
  - Available models: `tiny-en`, `tiny`, `base-en`, `base`, `small-en`, `small`, `medium-en`, `medium`, `large-v1`, `large-v2`, `large-v3`
  - Larger models = better accuracy but slower processing

**Statistics Tab:**

- View your current words per minute
- Track rolling average WPM (last 1000 samples)
- Monitor total words transcribed
- See total recording time

### WPM Display

After each transcription, the console displays:

```
WPM: 132.4 (27 words over 12.25s) | Avg: 118.7
```

- **Current WPM** - Speed of this transcription
- **Word count** - Number of words spoken
- **Duration** - Time from key press to release
- **Rolling average** - Average of your last 1000 transcriptions

## Building from Source

```bash
# Install Rust and dependencies
cargo build --release

# The executable will be in target/release/desk-talk.exe
```
