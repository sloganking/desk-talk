# DeskTalk v0.5.0 Release Notes

**Release Date:** October 1, 2025  
**Critical Security Update**

---

## 🔒 Security Fixes (CRITICAL)

### Fixed: Admin Token Exposure Vulnerability

- **Issue:** Previous versions bundled the Keygen admin token with customer installations
- **Impact:** Customers could extract the admin token and create unlimited free licenses
- **Fix:**
  - Created separate `.env.licenses.dist` file containing only public information
  - Admin token and policy IDs are no longer distributed to customers
  - Updated build configuration to bundle minimal config file only

**If you distributed v0.4.0 or earlier:** Your admin token was potentially exposed. Consider rotating it in the Keygen dashboard.

---

## 🎨 User Interface Improvements

### Fixed: Tray Icon Transparency

- System tray icon now displays correctly with transparent background
- No more white square around the microphone icon
- Works properly on both light and dark Windows themes

---

## 🛠️ Developer/Internal Changes

### Configuration System Improvements

- Made `admin_token`, `policy_trial`, and `policy_pro` optional in `KeygenConfig`
- Updated config loader to support both `.env.licenses` (dev) and `.env.licenses.dist` (production)
- Config lookup priority:
  1. `.env.licenses` (full config with admin token - for build machines)
  2. `.env.licenses.dist` (minimal config - for customer distribution)

### Test File Security

- Removed hardcoded production credentials from test files
- Test files now use environment variables for Keygen testing

### Documentation

- Added `SECURITY_FIX_README.md` - Complete security fix documentation
- Added `DEPLOYMENT_GUIDE.md` - Production deployment best practices
- Added `PRODUCTION_SECURITY_CHECKLIST.md` - Security status and checklist

---

## 📦 Installation

**Download:** `DeskTalk_0.5.0_x64-setup.exe`

**Requirements:**

- Windows 10/11 (64-bit)
- Valid DeskTalk license key

---

## 🔄 Upgrading from v0.4.0 or earlier

**Important:** Due to the security fix, we recommend a clean installation:

1. Uninstall your current version of DeskTalk
2. When prompted, choose to delete application data
3. Manually delete `C:\Program Files\DeskTalk` if it still exists
4. Install v0.5.0

This ensures the old `.env.licenses` file with admin token is completely removed.

---

## 🐛 Known Issues

None reported.

---

## 📝 Technical Details

### What's in the Installer

- ✅ `desk-talk.exe` (v0.5.0)
- ✅ `.env.licenses.dist` (public config only)
- ✅ Audio assets (beep sounds)
- ✅ Application icons
- ❌ **NO admin token** (secure!)

### License Validation

- Customer licenses are validated using their license key
- Machine fingerprinting prevents key sharing
- Admin token is only needed server-side for creating licenses

---

## 🔐 For Developers

### Building from Source

```bash
# Make sure you have the full .env.licenses on your build machine
# It should be in target/release/.env.licenses

# Build the installer
cargo tauri build

# The secure installer will be at:
# target/release/bundle/nsis/DeskTalk_0.5.0_x64-setup.exe
```

### Security Checklist

- [x] Admin token not in git
- [x] `.env.licenses.dist` contains only public info
- [x] Installer verified to exclude admin token
- [x] Test files use environment variables
- [x] Documentation updated

---

## 🙏 Acknowledgments

Special thanks to the developer for catching the admin token exposure issue before it became a widespread problem!

---

## 📞 Support

**Website:** https://desktalk.app  
**Repository:** https://github.com/sloganking/desk-talk  
**Issues:** https://github.com/sloganking/desk-talk/issues

For licensing issues, check your Keygen dashboard or contact support.
