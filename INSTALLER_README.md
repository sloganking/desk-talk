# DeskTalk Windows Installer

## 📦 Installer Build Complete!

The Windows installer has been successfully built and is ready for distribution.

### **Installer Location:**

```
target/release/bundle/nsis/DeskTalk_0.2.0_x64-setup.exe
```

**File Size:** ~4.3 MB  
**Type:** NSIS Installer (industry-standard Windows installer)

---

## 🚀 What the Installer Does

### **Installation Process:**

1. **Installs DeskTalk** to `C:\Program Files\DeskTalk\`
2. **Creates Start Menu shortcut**
3. **Registers the application** in Windows Add/Remove Programs
4. **Bundles all dependencies:**
   - Audio assets (`tick.mp3`, `failed.mp3`)
   - Application icon
   - All required DLLs

### **What's NOT Bundled (User Must Install):**

- **FFmpeg** - Required for audio processing

  - User needs to install separately: https://ffmpeg.org/download.html
  - Or use: `winget install Gyan.FFmpeg`
  - Or use: `scoop install ffmpeg`

- **OpenAI API Key** - Optional (for cloud transcription)
  - User provides in settings after installation

---

## 📋 Installation Instructions for Users

### **To Install:**

1. Run `DeskTalk_0.2.0_x64-setup.exe`
2. Follow the installer prompts
3. Launch DeskTalk from Start Menu
4. Install FFmpeg (if not already installed)
5. Activate your license key
6. Configure PTT key and API settings

### **To Uninstall:**

1. Go to Settings → Apps → Installed Apps
2. Find "DeskTalk"
3. Click Uninstall

---

## 🛠️ Technical Details

### **Installer Configuration:**

- **Type:** NSIS (Nullsoft Scriptable Install System)
- **Install Mode:** Per-machine (requires admin)
- **Compression:** LZMA (maximum compression)
- **Language:** English
- **64-bit only** (x64 architecture)

### **Application Details:**

- **Version:** 0.2.0
- **Publisher:** DeskTalk
- **License:** See LICENSE file
- **Homepage:** https://desktalk.app

### **Bundled Files:**

```
DeskTalk/
├── desk-talk.exe        (Main application)
├── assets/
│   ├── tick.mp3        (Success sound)
│   └── failed.mp3      (Error sound)
└── ...DLLs and resources
```

---

## 🔧 Building the Installer (For Developers)

### **Prerequisites:**

```bash
# Install Tauri CLI
cargo install tauri-cli --locked

# Install Rust (if not already installed)
# https://rustup.rs/
```

### **Build Command:**

```bash
cargo tauri build
```

### **Output:**

- **Installer:** `target/release/bundle/nsis/DeskTalk_0.2.0_x64-setup.exe`
- **Standalone EXE:** `target/release/desk-talk.exe`

---

## ⚠️ Known Limitations

### **Current Version (v0.2.0):**

1. **FFmpeg Not Bundled**

   - Users must install FFmpeg separately
   - Future: Bundle ffmpeg.exe with installer

2. **No Code Signing**

   - Windows will show "Unknown Publisher" warning
   - Users must click "More info" → "Run anyway"
   - Future: Purchase code signing certificate

3. **No Auto-Update**

   - Users must manually download new versions
   - Future: Implement Tauri updater

4. **Windows Only**
   - Only Windows x64 supported currently
   - Future: macOS and Linux builds

---

## 📝 Distribution Checklist

Before distributing to users:

- [x] Installer builds successfully
- [x] Application runs after installation
- [x] Uninstaller works correctly
- [ ] Test on clean Windows install (no dev tools)
- [ ] Test FFmpeg installation guidance
- [ ] Test license activation flow
- [ ] Test Stripe → Email → Activation workflow
- [ ] Create user documentation
- [ ] Upload to distribution server
- [ ] Update website download link

---

## 🐛 Troubleshooting

### **"Windows Protected Your PC" Warning:**

- Click "More info"
- Click "Run anyway"
- This is normal for unsigned applications

### **"FFmpeg not found" Error:**

- Install FFmpeg: https://ffmpeg.org/download.html
- Add FFmpeg to PATH
- Restart DeskTalk

### **Installation Fails:**

- Run installer as Administrator
- Check antivirus isn't blocking it
- Ensure enough disk space (~50 MB)

---

## 📧 Support

For installation issues, contact: support@desktalk.app  
For license/billing issues: billing@desktalk.app

---

**Last Updated:** October 1, 2025  
**Installer Version:** 0.2.0  
**Build System:** Tauri v2 + NSIS
