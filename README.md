# desk-talk

Transcription for your desktop.

A modern GUI application that records what you say when you press a button down, and types what you said when you release it.

> [!IMPORTANT]
> ⚠️ This video contains sound and is intended to be listened to with audio on. ⚠️

https://github.com/sloganking/desk-talk/assets/16965931/e5da605b-3a9d-4394-b4ec-a3de65605a65

## Features

✨ **Push-to-Talk Transcription** - Hold a key, speak, release to paste  
⚡ **Realtime Streaming** - Types text live *as you speak* via `gpt-realtime-whisper`  
✒️ **Smart End Punctuation** - An AI picks the correct ending mark (. ? !, language-aware)  
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

## Transcription modes

DeskTalk has two ways to turn your speech into text. Pick one in **General → Transcription Mode**. The out-of-the-box default is **Realtime** (with `xhigh` accuracy and smart end punctuation) so it "just works" with the least fuss; switch to Standard or dial settings down if you want to save on API cost.

### Standard (record, then transcribe)

Records your audio to a temporary file while you hold the key, then transcribes
the whole thing once you release. Highest fidelity, since the model sees the
entire recording at once. Supports **Parallel Racing** (send multiple requests
at once and use the fastest/most reliable result).

### Realtime (types as you speak)

Streams your microphone to OpenAI's `gpt-realtime-whisper` model over a
WebSocket and types the words into the focused window **live, as you talk** —
much lower perceived latency and a visual sense of progress. Requires the OpenAI
API (not available in local mode).

- **Accuracy / latency** is tunable with the **delay** setting:
  `minimal → low → medium → high → xhigh`. Higher gives the model more audio
  context before it commits text (better accuracy, text trails a bit further
  behind your voice). Default: `xhigh`.
- A debug log of each realtime session is written next to your config at
  `%APPDATA%\desk-talk\desk-talk\config\realtime.log`.

## End punctuation

A single setting controls the mark at the end of each utterance (the old
mutually-exclusive "always end with period" and "smart punctuation" toggles are
now one choice):

- **None** – leave the ending exactly as transcribed.
- **Period** – add a plain `.` if it doesn't already end with `.`, `?`, or `!`.
- **Smart** (default) – a cheap, fast model (`gpt-4o-mini`) picks the correct
  ending mark for what you said — `.`, `?`, `!`, or the appropriate mark for your
  language. It works in **both** transcription modes and requires the OpenAI API.

Smart mode **skips the LLM call entirely** when the text already ends with a
terminal mark — detecting that punctuation is present is a trivial local check
and needs no model — so you only pay for an API call when a mark is actually
missing.

## Command-line flags

The GUI app reads a few flags at launch that **override** the saved settings for
that run (handy for launchers/scripts). If a flag isn't passed, the saved
setting is used.

| Flag | Description |
| --- | --- |
| `--realtime` / `--no-realtime` | Force realtime streaming on / off |
| `--realtime-delay <level>` | `minimal`, `low`, `medium`, `high`, or `xhigh` |
| `--end-punctuation <mode>` | `none`, `period`, or `smart` |
| `--parallel <n>` | Number of parallel requests to race (Standard mode), 1–5 |

`--period`, `--smart-punctuation`, and `--no-smart-punctuation` are kept as
aliases for `--end-punctuation period`, `--end-punctuation smart`, and
`--end-punctuation none` respectively.

Example:

```bash
desk-talk.exe --realtime --realtime-delay xhigh --end-punctuation smart
```

## Building from Source

```bash
# Install Rust and dependencies
cargo build --release

# The executable will be in target/release/desk-talk.exe
```
