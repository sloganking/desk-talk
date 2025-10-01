# Debug Test Guide

## 🔧 **FIXES APPLIED:**

1. **REMOVED CRASHING KEY DETECTION** - The "Detect Key Press" button is now hidden (it was causing crashes)
2. **ADDED DEBUG LOGGING** - The terminal will now show exactly what's happening
3. **You'll manually select the key** from the dropdown instead

---

## 🧪 **TEST PROCEDURE:**

### Step 1: Restart the App

1. **Quit the current app** (right-click tray icon → Quit)
2. **Run from terminal** so you can see the debug output:
   ```powershell
   .\target\release\desk-talk.exe
   ```

### Step 2: Configure Settings

1. **Click the purple tray icon** to open settings
2. **General Tab:**
   - **PTT Key dropdown**: Select `F14` (or any key you want)
   - **Audio Device**: Leave as "Default Device" (or select your mic)
3. **Transcription Tab:**
   - Select "OpenAI API (Cloud)"
   - **Enter your OpenAI API key** (starts with `sk-`)
   - Click "Validate" to check it
4. **Click "Save Settings"**
   - Status should show "Settings saved successfully!"

### Step 3: Start Transcription

1. **Click "Start Transcription"** button
2. **WATCH THE TERMINAL** - You should see:
   ```
   Starting transcription engine...
   Config - PTT Key: Some(F14)
   Config - Device: default
   Config - Use Local: false
   Config - Has API Key: true
   Transcription engine using PTT key: F14
   Configuration validated successfully
   Event listener thread started
   Key handler thread started, waiting for PTT key: F14
   Transcription engine fully initialized - listening for key presses...
   Transcription engine started successfully!
   ```

### Step 4: Test Recording

1. **Open Notepad** (or any text editor)
2. **Click in the text area**
3. **Press and HOLD your PTT key** (F14)
4. **Speak:** "Hello world, this is a test"
5. **Release the key**

### Step 5: Check Terminal Output

**When you press the key, you should see:**

```
PTT key pressed - starting recording
Recording started successfully
Input device: [Your Microphone Name]
```

**When you release the key, you should see:**

```
PTT key released - stopping recording
WPM: [number] | Avg: [number] | Total: [number] words
```

**Then your text should appear in Notepad!**

---

## 🐛 **WHAT TO REPORT:**

### If it doesn't work, tell me:

1. **What appears in the terminal when you click "Start Transcription"?**

   - Does it show "Transcription engine started successfully!"?
   - Any error messages?

2. **What appears when you press your PTT key?**

   - Does it show "PTT key pressed"?
   - Does it show "Recording started successfully"?

3. **What appears when you release the key?**

   - Does it show "PTT key released"?
   - Any errors about transcription?

4. **Does the app crash?**
   - If yes, at what point?

---

## ✅ **EXPECTED BEHAVIOR:**

- App should NOT crash
- Terminal should show all the debug messages
- Pressing PTT key should start recording
- Releasing PTT key should transcribe and type text
- Text should appear in Notepad

---

## 🔍 **COMMON ISSUES:**

**"No push-to-talk key configured"**

- Make sure you selected a key from the dropdown
- Make sure you clicked "Save Settings"
- Try restarting the app

**"No OpenAI API key configured"**

- Make sure you entered your API key in the Transcription tab
- Click "Validate" to check it
- Make sure it starts with `sk-`
- Click "Save Settings"

**No debug output in terminal**

- Make sure you're running from the command line, not by double-clicking
- The exe must be run from a terminal window to see output

---

**Run the test and tell me what you see in the terminal!**
