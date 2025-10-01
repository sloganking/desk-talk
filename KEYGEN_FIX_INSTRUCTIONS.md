# Keygen License Activation - SOLUTION FOUND

## The Problem

License activation was failing with 401 Unauthorized because we were using the wrong authentication header format.

## The Root Cause

Keygen requires `Authorization: License <LICENSE_KEY>` for license-authenticated requests, NOT `Authorization: Bearer <LICENSE_KEY>`.

## Current Status

The code has been fixed to use the correct `License` auth header. However, activation will now fail with:

```
403 Forbidden: "License key authentication is not allowed by policy"
```

## What YOU Need to Do in Keygen Dashboard

1. Go to https://keygen.sh dashboard
2. Navigate to **Policies** → Select your policy (ID: `76f364d1-f4a1-4787-9744-48fb4335bb34`)
3. Find the **Authentication Strategy** setting
4. Change it from `TOKEN` to either:
   - `LICENSE` (only license keys can authenticate)
   - `MIXED` (both license keys and tokens can authenticate) ← **RECOMMENDED**
5. Save the policy

## After Changing the Policy

1. Close any running desk-talk.exe
2. Run: `cargo build`
3. Launch the app and try activating your license
4. It should now work!

## Technical Details

### What Changed in Code

- `src/license.rs` line 196: Changed from `.bearer_auth(license_key)` to `.header("Authorization", format!("License {}", license_key))`

### Auth Flow

1. **Validation**: POST to `/licenses/actions/validate-key` with license key in body (no auth needed)
2. **Machine Activation**: POST to `/machines` with:
   - `Authorization: License D5D8FE-381ADB-4E54A0-D2323D-1FAB90-V3` header
   - License ID in the relationships body
   - Machine fingerprint in attributes

This is the correct Keygen client-side activation flow.
