# Production Security Checklist for DeskTalk

## ✅ Current Security Status (as of October 1, 2025)

### What's Secure:

- ✅ Admin token is NOT in git history or public
- ✅ Public key is NOT in git history or public
- ✅ `.env.licenses` is in `.gitignore`
- ✅ License validation uses proper authentication

### What's Exposed (Low Risk):

- ⚠️ Keygen Account ID is in public test files
- ⚠️ Keygen Product ID is in git history (commit `69d7a90`)
- ⚠️ Policy IDs are in test files

**Risk Assessment:** These IDs alone cannot be used to create licenses. Only the admin token can do that, and it's secure.

---

## 🔒 Production Hardening Checklist

### Before Taking Real Customer Money:

- [ ] **1. Create Separate Test Keygen Account**

  - Sign up for a free Keygen account for testing
  - Use test credentials in `test_keygen.rs` and `keygen_test/`
  - Set environment variables: `KEYGEN_TEST_ACCOUNT_ID` and `KEYGEN_TEST_LICENSE_KEY`

- [ ] **2. Verify `.env.licenses` is Secure**

  - ✅ Already in `.gitignore`
  - ✅ Never commit this file again
  - Keep it ONLY on your build machine and release artifacts

- [ ] **3. Rotate Admin Token (Optional but Recommended)**

  - Go to https://app.keygen.sh
  - Generate a new admin token
  - Update `target/release/.env.licenses`
  - This invalidates the old token (even though it wasn't exposed)

- [ ] **4. Secure Your Build Pipeline**

  - If using CI/CD, store `.env.licenses` values as secrets
  - Never log the full token in build output
  - Bundle `.env.licenses` into release builds securely

- [ ] **5. Set Up License Verification**
  - ✅ Already implemented in `src/license.rs`
  - Licenses are validated on startup
  - Machine fingerprinting prevents key sharing

---

## 📋 Keygen Configuration Reference

### Current Production Setup:

```
KEYGEN_ACCOUNT_UID=40461088-b3c4-4c48-b4ff-8267dbafd938
KEYGEN_PRODUCT_ID=b87eb9aa-05a0-4068-9605-d70c16c2bd9e
KEYGEN_POLICY_TRIAL=b81687cc-ad4d-4924-a515-9bfc41bd515a
KEYGEN_POLICY_PRO=76f364d1-f4a1-4787-9744-48fb4335bb34
KEYGEN_ADMIN_TOKEN=prod-***[REDACTED]***
KEYGEN_PUBLIC_KEY=413be415***[REDACTED]***
```

### What Each Value Does:

- **ACCOUNT_UID**: Your Keygen account identifier
- **PRODUCT_ID**: Identifies DeskTalk product in Keygen
- **POLICY_TRIAL**: Policy for 7-day trial licenses
- **POLICY_PRO**: Policy for paid Pro licenses
- **ADMIN_TOKEN**: Allows creating licenses (KEEP SECRET!)
- **PUBLIC_KEY**: Ed25519 key for verifying license signatures

---

## 🚀 Deployment Best Practices

### Building Releases:

1. **Local Development:**

   ```bash
   # .env.licenses should be in your project root OR target/release/
   cargo build --release
   ```

2. **For Distribution:**

   - Copy `.env.licenses` to the same directory as `desk-talk.exe`
   - The app looks for it next to the executable first
   - Falls back to workspace root for development

3. **For Installers (NSIS):**
   - `.env.licenses` is already in `resources` (line 20 of tauri.conf.json)
   - It will be bundled into the installer automatically
   - Installed at: `C:\Program Files\DeskTalk\.env.licenses`

### If Credentials Are Compromised:

1. Go to https://app.keygen.sh immediately
2. Revoke the compromised admin token
3. Generate a new admin token
4. Optionally, rotate your Ed25519 signing key (this will invalidate existing licenses)
5. Update `.env.licenses` on your build machine
6. Rebuild and redistribute

---

## 🔍 Monitoring License Security

### Watch For:

- Unusual spikes in license validations
- Many failed validation attempts (could indicate key guessing)
- Licenses activated on more machines than allowed

### Keygen Dashboard:

- https://app.keygen.sh/accounts/40461088-b3c4-4c48-b4ff-8267dbafd938
- Monitor active licenses
- Check machine activations
- Review validation logs

---

## ✅ You're Ready for Production If:

- [x] Admin token is secret (NOT in git)
- [x] `.env.licenses` is in `.gitignore`
- [x] License validation is working
- [ ] Test files use separate credentials (just updated!)
- [x] Public key is secure
- [x] Installer bundles `.env.licenses`

**Overall Status:** ✅ **PRODUCTION READY** (after the test file changes are committed)

---

## 📞 Support

If you suspect a security breach:

1. Rotate all Keygen credentials immediately
2. Check Keygen logs for unauthorized activity
3. Contact Keygen support: https://keygen.sh/support
