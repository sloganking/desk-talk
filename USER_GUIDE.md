# DeskTalk User Guide

## 🎤 Voice-to-Text Transcription Made Easy

DeskTalk listens when you hold a button, transcribes what you said, and types it where your cursor is.

---

## 🚀 Quick Start (5 Minutes)

### **Step 1: Install DeskTalk**

1. Run `DeskTalk_0.2.0_x64-setup.exe`
2. Follow installer prompts
3. Launch from Start Menu

### **Step 2: Install FFmpeg**

**Option A - Winget (Easiest):**

```powershell
winget install Gyan.FFmpeg
```

**Option B - Scoop:**

```powershell
scoop install ffmpeg
```

**Option C - Manual:**

1. Download from https://ffmpeg.org/download.html
2. Extract to `C:\ffmpeg`
3. Add `C:\ffmpeg\bin` to PATH

### **Step 3: Get a License**

1. Open DeskTalk → **License** tab
2. Click **"Buy License"** ($100 one-time)
3. Complete payment via Stripe
4. Check your email for license key
5. Paste key in DeskTalk → Click **"Activate"**

### **Step 4: Get OpenAI API Key** (or use Local Model)

**For OpenAI (Recommended for Accuracy):**

1. Go to https://platform.openai.com/api-keys
2. Create new secret key
3. Copy the key (starts with `sk-...`)
4. Paste in DeskTalk → **Transcription** tab
5. Click **"Validate"**

**For Local Model (Offline, Free):**

1. Select **"Local Model"** in Transcription tab
2. Choose a model (e.g., `base.en`)
3. First use will download the model

### **Step 5: Set Up Push-to-Talk**

1. Open DeskTalk → **General** tab
2. Click dropdown next to "Push-to-Talk Key"
3. Select a key (e.g., `F15`, `F13`, `CapsLock`)
4. Click **"Save Settings"**

### **Step 6: Start Talking!**

1. Status should show **"Running"** (green)
2. Click in any text field
3. **Hold your PTT key** → Speak → **Release**
4. Your words appear! 🎉

---

## 📖 Detailed Setup

### **General Settings**

#### **Push-to-Talk Key**

- Choose an unused key (F13-F24 recommended)
- Avoid keys you use often (don't use Space, Enter, etc.)
- Best options:
  - `F15` - Rarely used function key
  - `F13` - Good if you have extended keyboard
  - `CapsLock` - If you never use it

#### **Audio Input Device**

- **"Default Device"** - Uses Windows default mic
- Or select specific microphone
- Test by recording and checking quality

#### **Text Output Options**

- **Capitalize first letter** - Start transcriptions with capital
- **Add space after** - Auto-space between transcriptions
- **Type characters instead of pasting** ⓘ
  - Slower but works in apps that block Ctrl+V
  - Leave off for best speed

#### **System Options**

- **Start with Windows** - Launch DeskTalk on boot
- **Start minimized to tray** - Hide window on startup
- **Dark mode** - Easy on the eyes

### **Transcription Settings**

#### **OpenAI API (Cloud)**

**Pros:**

- Most accurate transcription
- Handles accents, background noise
- Fast processing

**Cons:**

- Costs money (~$0.006 per minute)
- Requires internet
- Sends audio to OpenAI

**Setup:**

1. Get key from https://platform.openai.com/api-keys
2. Paste in field
3. Click "Validate" to test
4. Click "View Usage & Billing" to monitor costs

**Cost Example:**

- 1000 minutes = ~$6
- Average user: $2-5/month

#### **Local Model (Offline)**

**Pros:**

- Free forever
- Works offline
- Private (audio stays on your PC)

**Cons:**

- Less accurate than OpenAI
- Slower on older PCs
- Larger download (~1GB for best model)

**Models:**

- `tiny.en` - Fastest, least accurate (75 MB)
- `base.en` - Good balance (142 MB)
- `small.en` - Better accuracy (466 MB)
- `medium.en` - Best accuracy (1.5 GB)

### **License Tab**

#### **License Status**

- **Active** (Green) - Everything works
- **Expired** (Yellow) - Renew license
- **Inactive** (Red) - Need to activate

#### **Activate License**

1. Paste your license key
2. Click "Activate"
3. Key tied to this PC (3 devices max per license)

#### **Deactivate License**

- Click "Deactivate" to free up device slot
- Useful for switching PCs or testing

### **Statistics Tab**

Track your usage:

- **Total Words** - Words transcribed all-time
- **Average WPM** - Your speaking speed
- **Transcriptions** - Number of recordings
- **Recording Time** - Total time speaking

---

## 💡 Tips & Tricks

### **For Best Transcription:**

1. **Speak clearly** (but naturally)
2. **Use a good microphone** (not laptop built-in)
3. **Quiet environment** (reduce background noise)
4. **Hold button first, then speak** (avoid cutting off start)
5. **Pause briefly** before releasing (avoid cutting off end)

### **Common Use Cases:**

**Writing Emails:**

- Use PTT for long paragraphs
- Faster than typing
- Natural conversational tone

**Coding Comments:**

- Quickly document code
- Explain complex functions
- Add TODOs

**Documentation:**

- Draft documents quickly
- Brainstorm ideas
- Meeting notes

**Accessibility:**

- RSI/carpal tunnel relief
- Hands-free input
- Faster than typing

### **Keyboard Shortcuts:**

- **Click tray icon** - Open settings
- **Right-click tray icon** - Quick menu
  - Restart Engine
  - Stop Engine
  - Settings
  - Quit

### **Troubleshooting Recording:**

**No transcription appearing:**

1. Check status is "Running" (green)
2. Verify PTT key is configured
3. Ensure cursor is in a text field
4. Try longer recording (>1 second)

**Transcription is wrong:**

1. Speak more clearly
2. Try OpenAI instead of Local
3. Check microphone quality
4. Reduce background noise

**Sound plays but nothing types:**

1. Click in text field first
2. Check "Type characters" option
3. Try different application
4. Restart engine (right-click tray)

---

## ⚙️ Advanced Settings

### **Environment Variables**

DeskTalk respects these env vars:

- `OPENAI_API_KEY` - Override API key
- `RUST_LOG=debug` - Enable debug logging

### **Config File Location**

```
%APPDATA%\com.desk-talk.app\config.json
```

### **Statistics File**

```
%APPDATA%\com.desk-talk.app\statistics.json
```

### **Logs** (if enabled)

```
%APPDATA%\com.desk-talk.app\logs\
```

---

## 🐛 Troubleshooting

### **Installation Issues**

**"Windows protected your PC"**

- Click "More info" → "Run anyway"
- Normal for apps without code signing

**Installation fails**

- Run as Administrator
- Disable antivirus temporarily
- Check disk space (need ~50 MB)

### **FFmpeg Issues**

**"FFmpeg not found"**

```powershell
# Test if FFmpeg is installed
ffmpeg -version

# If not found, install:
winget install Gyan.FFmpeg

# Restart DeskTalk after installing
```

### **License Issues**

**"License validation failed"**

- Check internet connection
- Verify key was copied correctly
- Contact support@desktalk.app

**"Device already activated"**

- You've activated 3 devices max
- Deactivate on another PC first
- Or contact support for device reset

### **Transcription Issues**

**"Invalid API key"**

- Get new key from OpenAI
- Ensure you copied full key (starts with `sk-`)
- Check OpenAI account has credits

**Transcription too slow**

- Use `tiny.en` or `base.en` model
- Or switch to OpenAI API (faster)
- Close other heavy applications

**Poor accuracy**

- Use OpenAI instead of Local
- Upgrade to larger local model
- Improve microphone quality

### **Audio Issues**

**No recording**

- Check microphone permissions in Windows
- Select correct audio device in settings
- Test mic in Windows Sound settings

**Echo or feedback**

- Disable microphone monitoring
- Use headphones
- Reduce speaker volume

---

## 📊 Pricing

### **Software License**

- **Pro License:** $100 (one-time)
- **Includes:**
  - Unlimited transcriptions
  - 3 device activations
  - Lifetime updates
  - Email support

### **OpenAI API Costs** (Optional)

- **Whisper API:** ~$0.006/minute
- **Pay-as-you-go** (no subscription)
- **Example costs:**
  - Light user (100 min/month): ~$0.60
  - Medium user (500 min/month): ~$3.00
  - Heavy user (2000 min/month): ~$12.00

### **Local Model** (Free Alternative)

- **$0 forever**
- **No per-minute costs**
- **No internet required**

---

## 🔐 Privacy & Security

### **Data Privacy:**

- **Local Model:** Audio never leaves your PC
- **OpenAI API:** Audio sent to OpenAI for processing
  - See: https://openai.com/policies/privacy-policy
  - OpenAI doesn't train on API data (by default)

### **Stored Data:**

- **Config:** Settings only, no audio
- **Statistics:** Word counts, times
- **License:** License key (encrypted in Windows Credential Manager)
- **Audio:** Temporary files deleted after transcription

### **Network Usage:**

- **License validation:** Keygen.sh (on startup/activation)
- **OpenAI API:** Only when transcribing (if using cloud)
- **Updates:** None (manual updates only for now)

---

## 📧 Support

### **Get Help:**

- **Email:** support@desktalk.app
- **Billing:** billing@desktalk.app

### **Reporting Bugs:**

Include:

1. Windows version
2. DeskTalk version (see About)
3. Error message
4. Steps to reproduce

---

## 🎓 Video Tutorials

**Getting Started with DeskTalk:**
https://youtu.be/SzPE_AE0eEo

**How to Get an OpenAI API Key:**
https://youtu.be/SzPE_AE0eEo

---

## 🚀 Updates

**Current Version:** 0.2.0  
**Last Updated:** October 1, 2025

**What's New:**

- Initial public release
- Keygen licensing system
- Dark mode support
- Statistics tracking
- Error sound feedback

**Coming Soon:**

- Auto-updates
- macOS support
- Bundled FFmpeg
- Code signing

---

## 📜 License

Copyright © 2025 DeskTalk  
See LICENSE file for details

---

**Enjoy faster, hands-free typing with DeskTalk!** 🎤✨
