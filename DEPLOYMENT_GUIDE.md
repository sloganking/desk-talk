# DeskTalk Deployment Guide

## Building for Production

### Prerequisites:

- Full `.env.licenses` file in `target/release/` with admin token (for your build machine only)
- `.env.licenses.dist` committed to git (minimal config for customers)

### Build Steps:

```bash
# 1. Make sure you have the full config on your build machine
# Copy target/release/.env.licenses or create it with:
KEYGEN_ACCOUNT_UID=40461088-b3c4-4c48-b4ff-8267dbafd938
KEYGEN_PRODUCT_ID=b87eb9aa-05a0-4068-9605-d70c16c2bd9e
KEYGEN_POLICY_TRIAL=b81687cc-ad4d-4924-a515-9bfc41bd515a
KEYGEN_POLICY_PRO=76f364d1-f4a1-4787-9744-48fb4335bb34
KEYGEN_ADMIN_TOKEN=prod-[YOUR_TOKEN_HERE]
KEYGEN_PUBLIC_KEY=413be41504f08c9c04cff91df93778b0e91697e963f098d5683cd1cf110ba23c

# 2. Build the release installer
cargo build --release

# 3. Create the NSIS installer
cargo tauri build

# The installer is at: target/release/bundle/nsis/DeskTalk_0.4.0_x64-setup.exe
```

### What Gets Bundled:

✅ **Included in installer:**

- `desk-talk.exe`
- `.env.licenses.dist` (minimal config, NO admin token)
- Assets (beep sounds, etc.)
- Icons

❌ **NOT included in installer:**

- `.env.licenses` (full config with admin token)
- Build artifacts
- Source code

### Verification:

After building, verify the security:

```bash
# Extract the installer (or install it) and check the config file
# It should be .env.licenses.dist with ONLY:
# - KEYGEN_ACCOUNT_UID
# - KEYGEN_PRODUCT_ID
# - KEYGEN_PUBLIC_KEY
#
# NO admin_token!
```

---

## Deployment Environments

### 1. Development Machine

- Uses `.env.licenses` (full config)
- Can create licenses if needed
- Run: `cargo run` or `cargo build`

### 2. Build/CI Machine

- Needs `.env.licenses` (full config) in `target/release/`
- Creates production installers
- Keep admin token secret!

### 3. Customer Machines

- Receives `.env.licenses.dist` (minimal config)
- Can validate licenses
- **Cannot create licenses** (no admin token)

---

## Creating Licenses for Customers

You'll need to create licenses manually or via API using your admin token.

### Option 1: Keygen Dashboard (Easiest)

1. Go to https://app.keygen.sh
2. Navigate to your product
3. Click "Create License"
4. Select policy (Trial or Pro)
5. Send the license key to your customer

### Option 2: API (Automated)

Use your admin token to create licenses via Keygen API:

```bash
curl -X POST https://api.keygen.sh/v1/accounts/40461088-b3c4-4c48-b4ff-8267dbafd938/licenses \
  -H "Authorization: Bearer prod-[YOUR_TOKEN]" \
  -H "Content-Type: application/vnd.api+json" \
  -d '{
    "data": {
      "type": "licenses",
      "attributes": {
        "policyId": "76f364d1-f4a1-4787-9744-48fb4335bb34"
      }
    }
  }'
```

### Option 3: Build Your Own License Generator

Create a secure server-side tool that:

1. Authenticates you (e.g., password)
2. Uses your admin token to call Keygen API
3. Generates and emails licenses to customers
4. **Never exposes the admin token!**

---

## Security Checklist

Before distributing to customers:

- [ ] `.env.licenses.dist` contains ONLY public info
- [ ] `.env.licenses.dist` is committed to git
- [ ] `.env.licenses` (with admin token) is in `.gitignore`
- [ ] `tauri.conf.json` bundles `.env.licenses.dist`, not `.env.licenses`
- [ ] Admin token is kept secret on build machine only
- [ ] Test that the installer doesn't contain admin token

---

## Updating Credentials

If you need to rotate your Keygen credentials:

### Rotating Admin Token:

1. Go to Keygen dashboard
2. Generate new admin token
3. Update `target/release/.env.licenses` on build machine
4. **Do not distribute to customers!**
5. Old installers still work (they don't use admin token)

### Rotating Public Key:

1. Generate new Ed25519 key pair in Keygen
2. Update `.env.licenses.dist` with new public key
3. Commit and push to git
4. **Rebuild all installers** (existing licenses won't validate with old key)
5. **This will invalidate existing customer licenses!**

---

## Troubleshooting

### Customer License Won't Validate:

1. Check they have internet connection (for Keygen API)
2. Verify license is ACTIVE in Keygen dashboard
3. Check machine activation count (max machines reached?)
4. Test the license key in your test environment

### Build Machine Can't Load Config:

```
Error: .env.licenses not found
```

**Solution:** Create `target/release/.env.licenses` with full config including admin token.

### Installer Contains Admin Token:

🚨 **CRITICAL SECURITY ISSUE!**

1. Check `tauri.conf.json` - should reference `.env.licenses.dist`
2. Verify `.env.licenses.dist` doesn't have admin token
3. Rebuild installer
4. Extract and verify no admin token in distributed files

---

## Distribution Checklist

Before releasing a new version:

- [ ] Tested license validation locally
- [ ] Tested license activation with fresh license
- [ ] Verified installer doesn't contain `.env.licenses` (only `.env.licenses.dist`)
- [ ] Tested on clean Windows installation
- [ ] Updated version number in `Cargo.toml` and `tauri.conf.json`
- [ ] Built release installer: `cargo tauri build`
- [ ] Uploaded installer to distribution server
- [ ] Updated website download link

---

## Support

If a customer has licensing issues:

1. **Check Keygen Dashboard:**

   - Is their license ACTIVE?
   - Are they within machine limit?
   - Has it expired?

2. **Common Solutions:**

   - Deactivate a machine to free up a slot
   - Extend expiration date
   - Increase machine limit

3. **Emergency:**
   - Generate a new license key
   - Send to customer
   - They activate with new key

**Remember:** Never share your admin token with anyone, including customers!
