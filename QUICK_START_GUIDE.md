# Quick Start Guide - DeskTalk GUI

## 🔄 How to Restart the App After Updates

1. **Find the purple circle icon in your system tray** (bottom-right, near the clock)
2. **Right-click it**
3. Click **"Quit"**
4. The app will close
5. Run it again: Double-click `target\release\desk-talk.exe`

---

## ✅ First-Time Setup (Follow these steps!)

### Step 1: Configure Your PTT Key

1. **Open Settings**: Click the purple tray icon
2. Go to the **General** tab
3. Click **"Detect Key Press"** button
4. Press the key you want to use (e.g., F13, ScrollLock, etc.)
5. The key will appear in the dropdown

### Step 2: Choose Transcription Mode

**Option A: OpenAI API (Cloud - Recommended)**

1. Go to the **Transcription** tab
2. Make sure "OpenAI API (Cloud)" is selected
3. Enter your OpenAI API key (starts with `sk-`)
   - Get one at: https://platform.openai.com/api-keys
4. Click **"Validate"** to check it

**Option B: Local Model (Offline)**

1. Go to the **Transcription** tab
2. Select "Local Model (Offline)"
3. Choose a model from the dropdown (start with "Tiny" for testing)
4. Model will download automatically on first use

### Step 3: Select Audio Device (Optional)

1. In the **General** tab
2. Click **"Refresh"** next to the audio device dropdown
3. Select your microphone

### Step 4: Save Settings

1. Click **"Save Settings"** at the bottom
2. You should see "Settings saved successfully!"

### Step 5: Start Transcription!

1. Click **"Start Transcription"**
2. **Hold down your PTT key** and speak
3. **Release the key** - your text will be typed where your cursor is!

---

## 🎯 Testing It Out

1. Open Notepad or any text editor
2. Click in the text area
3. Hold your PTT key
4. Say: "Hello world, this is a test"
5. Release the key
6. Wait a moment (you'll hear a ticking sound while processing)
7. Your text should appear!

---

## 🐛 Troubleshooting

**"No transcription" or nothing happens:**

- Make sure you held the key for at least 0.5 seconds
- Check that your microphone is working
- Verify your API key is correct (for OpenAI mode)

**"Error starting transcription":**

- Make sure you selected a PTT key
- For OpenAI: Enter a valid API key
- For Local: Select a model

**Can't find the tray icon:**

- Look in the bottom-right corner of your screen
- Click the up arrow (^) to show hidden icons
- Look for a purple circle

---

## 📊 View Your Stats

Click the **Statistics** tab to see:

- Total words transcribed
- Average words per minute (WPM)
- Total recording time
- Number of sessions

---

## 💡 Pro Tips

- **Capitalize first letter**: Enable in General tab
- **Add space after**: Useful for continuous dictation
- **Type instead of paste**: Use if pasting doesn't work in your app

---

**Need help?** The new "Detect Key Press" button makes setup much easier!
