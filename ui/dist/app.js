// Get the invoke function from Tauri API (supports both v1 and v2)
const invoke = window.__TAURI_INTERNALS__?.invoke || window.__TAURI__?.core?.invoke || window.__TAURI__?.tauri?.invoke;

if (!invoke) {
    console.error('Tauri invoke function not found!');
    alert('ERROR: Tauri API not loaded. The app may not work correctly.');
}

let cachedApiKey = '';
let licenseInfo = {
    status: 'Unknown',
    plan: 'Unknown',
    key: null,
    expiresAt: null,
    maxMachines: null,
    machinesUsed: null,
    hasLicense: false,
};
let currentPttKey = null;
let trialCountdownInterval = null;

// Format time remaining in human-readable format
function formatTimeRemaining(ms) {
    if (ms <= 0) {
        return 'Expired';
    }

    const seconds = Math.floor(ms / 1000);
    const now = Math.floor(Date.now() / 1000);

    // Compute target expiration Unix timestamp
    const expirationUnix = now + seconds;

    return invoke('format_trial_remaining', { expiration: new Date(expirationUnix * 1000).toISOString() })
        .catch(err => {
            console.warn('Failed to format trial remaining:', err);
            // Fallback to manual formatting
            const minutes = Math.floor(seconds / 60);
            const hours = Math.floor(minutes / 60);
            const days = Math.floor(hours / 24);

            const remainingHours = hours % 24;
            const remainingMinutes = minutes % 60;
            const remainingSeconds = seconds % 60;

            const parts = [];
            if (days > 0) parts.push(`${days}d`);
            if (remainingHours > 0 || days > 0) parts.push(`${remainingHours}h`);
            if (remainingMinutes > 0 || hours > 0) parts.push(`${remainingMinutes}m`);
            parts.push(`${remainingSeconds}s`);

            return parts.join(' ');
        });
}

// Start live countdown for trial expiration
function startTrialCountdown(expirationDate, element, initialText) {
    // Stop existing countdown if any
    stopTrialCountdown();
    
    const updateCountdown = () => {
        const now = new Date().getTime();
        const expiration = new Date(expirationDate).getTime();
        const remaining = expiration - now;
        
        Promise.resolve(formatTimeRemaining(remaining)).then(text => {
            element.textContent = text;
        });
        
        // If expired, stop the countdown and refresh UI
        if (remaining <= 0) {
            stopTrialCountdown();
            element.textContent = 'Expired';
            // Refresh the entire license section to update status
            setTimeout(() => updateLicenseSection(), 1000);
        }
    };
    
    // Update immediately
    if (initialText) {
        element.textContent = initialText;
    } else {
        updateCountdown();
    }
    
    // Then update every second
    trialCountdownInterval = setInterval(updateCountdown, 1000);
}

// Stop the trial countdown
function stopTrialCountdown() {
    if (trialCountdownInterval) {
        clearInterval(trialCountdownInterval);
        trialCountdownInterval = null;
    }
}

function applyPttKeySelection() {
    if (!currentPttKey) {
        return;
    }
    const select = document.getElementById('pttKey');
    if (!select) {
        return;
    }
    const optionExists = Array.from(select.options).some(
        (option) => option.value === currentPttKey
    );
    if (!optionExists) {
        const option = document.createElement('option');
        option.value = currentPttKey;
        option.textContent = currentPttKey;
        select.appendChild(option);
    }
    select.value = currentPttKey;
}

async function updateEngineStatus(isRunning, errorMessage = null) {
    const statusEl = document.getElementById('engineStatus');
    const errorEl = document.getElementById('engineError');
    if (!statusEl) return;
    
    if (isRunning) {
        statusEl.textContent = 'Running';
        statusEl.className = 'status-badge active';
        if (errorEl) {
            errorEl.style.display = 'none';
            errorEl.textContent = '';
        }
    } else {
        statusEl.textContent = 'Stopped';
        statusEl.className = 'status-badge inactive';
        
        // Check why it's stopped and show appropriate error
        if (!errorMessage) {
            errorMessage = await getEngineStopReason();
        }
        
        if (errorEl && errorMessage) {
            errorEl.textContent = `⚠️ ${errorMessage} ⚠️`;
            errorEl.style.display = 'block';
        }
    }
}

async function getEngineStopReason() {
    try {
        const config = await invoke('get_config');
        
        // Check for license OR valid trial
        let hasValidAccess = false;
        
        if (config.license_key) {
            hasValidAccess = true;
        } else {
            // No license, check for active trial
            try {
                const trialStatus = await invoke('get_trial_status');
                if (trialStatus.is_trial && !trialStatus.expired) {
                    hasValidAccess = true;
                } else if (trialStatus.is_trial && trialStatus.expired) {
                    return 'Trial period has expired. Please purchase a license to continue using DeskTalk.';
                }
            } catch (e) {
                console.warn('Failed to check trial status:', e);
            }
        }
        
        if (!hasValidAccess) {
            return 'No active license or trial. Please activate a license or start a trial.';
        }
        
        // Check PTT key
        if (!config.ptt_key) {
            return 'No push-to-talk key configured. Select a key in General settings.';
        }
        
        // Check API key for OpenAI mode
        if (!config.use_local && !config.api_key) {
            return 'No OpenAI API key configured. Enter your API key in Transcription settings.';
        }
        
        // Check local model for local mode
        if (config.use_local && !config.local_model) {
            return 'No local model selected. Choose a model in Transcription settings.';
        }
        
        // All required config is present, but engine isn't running
        // This means it failed to start or was manually stopped
        return 'Engine stopped. Save Settings to restart, or check for errors above.';
    } catch (error) {
        console.error('Failed to get stop reason:', error);
        return 'Engine stopped.';
    }
}

// Tab switching
document.querySelectorAll('.tab-button').forEach(button => {
    button.addEventListener('click', () => {
        const tabName = button.dataset.tab;
        
        // Update buttons
        document.querySelectorAll('.tab-button').forEach(b => b.classList.remove('active'));
        button.classList.add('active');
        
        // Update content
        document.querySelectorAll('.tab-content').forEach(c => c.classList.remove('active'));
        document.getElementById(tabName).classList.add('active');
        
        // Load statistics when switching to stats tab
        if (tabName === 'stats') {
            loadStatistics();
        }
    });
});

// Transcription mode switching
document.querySelectorAll('input[name="transcriptionMode"]').forEach(radio => {
    radio.addEventListener('change', (e) => {
        const isLocal = e.target.value === 'local';
        document.getElementById('openaiSection').style.display = isLocal ? 'none' : 'block';
        document.getElementById('localSection').style.display = isLocal ? 'block' : 'none';
    });
});

// Load configuration on startup
async function loadConfig() {
    try {
        const config = await invoke('get_config');
        console.log('Loaded config from backend:', config);
        console.log('Config.api_key:', config.api_key ? `EXISTS (${config.api_key.length} chars)` : 'MISSING');
        
        // General settings
        if (config.ptt_key) {
            currentPttKey = config.ptt_key;
            applyPttKeySelection();
        }
        document.getElementById('audioDevice').value = config.device || 'default';
        document.getElementById('capFirst').checked = config.cap_first || false;
        document.getElementById('space').checked = config.space || false;
        document.getElementById('typeChars').checked = config.type_chars || false;
        document.getElementById('autoStart').checked = config.auto_start || false;
        document.getElementById('startMinimized').checked = config.start_minimized || false;
        document.getElementById('darkMode').checked = config.dark_mode || false;
        
        // Apply dark mode immediately
        if (config.dark_mode) {
            document.body.classList.add('dark-mode');
        } else {
            document.body.classList.remove('dark-mode');
        }
        
        // Transcription settings
        const isLocal = config.use_local || false;
        document.getElementById('modeOpenAI').checked = !isLocal;
        document.getElementById('modeLocal').checked = isLocal;
        document.getElementById('openaiSection').style.display = isLocal ? 'none' : 'block';
        document.getElementById('localSection').style.display = isLocal ? 'block' : 'none';
        
        const apiKeyField = document.getElementById('apiKey');
        console.log('API key field element:', apiKeyField ? 'FOUND' : 'NOT FOUND');
        if (config.api_key) {
            cachedApiKey = config.api_key;
            apiKeyField.value = config.api_key;
            console.log('✓ API key loaded from backend (length:', config.api_key.length, ')');
            console.log('✓ API key field value after set:', apiKeyField.value);
            console.log('✓ API key field display:', apiKeyField.style.display);
        } else {
            console.warn('✗ No API key in loaded config');
            apiKeyField.value = ''; // Clear the field
        }

        if (config.local_model) {
            document.getElementById('localModel').value = config.local_model;
        }

        await refreshLicenseStatus();

        const running = await invoke('is_running');
        updateEngineStatus(running);
        
        console.log('Configuration loaded:', config);
    } catch (error) {
        console.error('Error loading configuration:', error);
        showStatus('Error loading configuration: ' + error, 'error');
    }
}

// Load PTT keys
async function loadPTTKeys() {
    try {
        const keys = await invoke('get_available_ptt_keys');
        const select = document.getElementById('pttKey');
        select.innerHTML = '<option value="">Select a key...</option>';
        keys.forEach(key => {
            const option = document.createElement('option');
            option.value = key;
            option.textContent = key;
            select.appendChild(option);
        });
        applyPttKeySelection();
    } catch (error) {
        console.error('Error loading PTT keys:', error);
    }
}

// Load audio devices
async function loadAudioDevices() {
    try {
        const devices = await invoke('get_audio_devices');
        const select = document.getElementById('audioDevice');
        select.innerHTML = '<option value="default">Default Device</option>';
        devices.forEach(device => {
            const option = document.createElement('option');
            option.value = device;
            option.textContent = device;
            select.appendChild(option);
        });
    } catch (error) {
        showStatus('Error loading audio devices: ' + error, 'error');
    }
}

// Load statistics
async function loadStatistics() {
    try {
        const stats = await invoke('get_statistics');
        document.getElementById('totalWords').textContent = stats.total_words || 0;
        document.getElementById('avgWPM').textContent = (stats.average_wpm || 0).toFixed(1);
        document.getElementById('sessionCount').textContent = stats.session_count || 0;

        const totalSeconds = stats.total_recording_time_secs || 0;
        const minutes = Math.floor(totalSeconds / 60);
        const seconds = Math.floor(totalSeconds % 60);
        if (minutes > 0) {
            document.getElementById('totalTime').textContent = `${minutes}m ${seconds}s`;
        } else {
            document.getElementById('totalTime').textContent = `${seconds}s`;
        }
    } catch (error) {
        console.error('Error loading statistics:', error);
    }
}

// Save configuration
async function saveConfig() {
    try {
        const pttKeyValue = document.getElementById('pttKey').value;
        
        // If PTT key is cleared/unset, stop the engine
        if (!pttKeyValue) {
            const wasRunning = await invoke('is_running');
            if (wasRunning) {
                await invoke('stop_engine');
                await updateEngineStatus(false);
            }
            
            // Still save the config with empty PTT key
            const config = {
                ptt_key: null,
                special_ptt_key: null,
                device: document.getElementById('audioDevice').value,
                use_local: document.getElementById('modeLocal').checked,
                local_model: document.getElementById('localModel').value || null,
                cap_first: document.getElementById('capFirst').checked,
                space: document.getElementById('space').checked,
                type_chars: document.getElementById('typeChars').checked,
                auto_start: document.getElementById('autoStart').checked,
                start_minimized: document.getElementById('startMinimized').checked,
                dark_mode: document.getElementById('darkMode').checked,
                api_key: document.getElementById('apiKey').value || cachedApiKey || null,
                license_key: null,
                license_plan: null,
                license_id: null,
                trial_expiration: null,
                trial_started: false,
                machine_id: "",
            };
            
            await invoke('save_config', { incoming: config });
            await updateEngineStatus(false, 'No push-to-talk key configured. Select a key in General settings.');
            showStatus('PTT key cleared and engine stopped.', 'success');
            return true;
        }
        
        const isLocal = document.getElementById('modeLocal').checked;
        const apiKeyInput = document.getElementById('apiKey');
        const apiKey = apiKeyInput.value || cachedApiKey;
        if (apiKeyInput.value) {
            cachedApiKey = apiKeyInput.value;
        }
        
        console.log('=== SAVING CONFIG ===');
        console.log('PTT Key:', pttKeyValue);
        console.log('API Key length:', apiKey ? apiKey.length : 0);
        console.log('API Key value:', apiKey ? '(exists)' : '(empty)');
        
        if (!isLocal && !apiKey) {
            showStatus('Please enter an OpenAI API key or switch to Local mode!', 'error');
            return false;
        }
        
        if (isLocal && !document.getElementById('localModel').value) {
            showStatus('Please select a local model!', 'error');
            return false;
        }
        
    const config = {
            ptt_key: pttKeyValue,
            special_ptt_key: null,
            device: document.getElementById('audioDevice').value,
            use_local: isLocal,
            local_model: document.getElementById('localModel').value || null,
            cap_first: document.getElementById('capFirst').checked,
            space: document.getElementById('space').checked,
            type_chars: document.getElementById('typeChars').checked,
            auto_start: document.getElementById('autoStart').checked,
            start_minimized: document.getElementById('startMinimized').checked,
            dark_mode: document.getElementById('darkMode').checked,
            api_key: apiKey || null,
            // These fields are managed by backend, send null/default so serde doesn't fail
            license_key: null,
            license_plan: null,
            license_id: null,
            trial_expiration: null,
            trial_started: false,
            machine_id: "", // Backend will preserve the real value
        };
        
        console.log('Config payload being sent:', JSON.stringify({ ...config, api_key: apiKey ? '(hidden)' : null }, null, 2));
        
        // Test OpenAI API key if using OpenAI mode
        if (!isLocal && apiKey) {
            try {
                showStatus('Testing API key...', '');
                await invoke('test_openai_key', { apiKey });
                console.log('API key test passed');
            } catch (error) {
                console.error('API key test failed:', error);
                try {
                    const runningBeforeStop = await invoke('is_running');
                    if (runningBeforeStop) {
                        await invoke('stop_engine');
                    }
                } catch (stopError) {
                    console.error('Failed to stop engine after API key test failure:', stopError);
                }
                await updateEngineStatus(false, error.toString());
                showStatus('Settings saved, but API key is invalid: ' + error, 'error');
                await invoke('save_config', { incoming: config });
                return true;
            }
        }
        
        await invoke('save_config', { incoming: config });
        
        // Auto-restart engine if it was running
        const wasRunning = await invoke('is_running');
        if (wasRunning) {
            try {
                await invoke('stop_engine');
                await updateEngineStatus(false);
                await invoke('start_engine');
                // Verify it's actually running after start
                const isNowRunning = await invoke('is_running');
                await updateEngineStatus(isNowRunning);
                if (isNowRunning) {
                    showStatus('Settings saved and engine restarted!', 'success');
                } else {
                    showStatus('Settings saved, but engine failed to start.', 'error');
                }
            } catch (error) {
                console.error('Failed to restart engine:', error);
                await updateEngineStatus(false);
                showStatus('Settings saved, but failed to restart: ' + error, 'error');
            }
        } else {
            // Try to start engine if it wasn't running
            try {
                await invoke('start_engine');
                // Verify it's actually running after start
                const isNowRunning = await invoke('is_running');
                await updateEngineStatus(isNowRunning);
                if (isNowRunning) {
                    showStatus('Settings saved and engine started!', 'success');
                } else {
                    showStatus('Settings saved, but engine failed to start.', 'error');
                }
            } catch (error) {
                console.error('Failed to start engine:', error);
                await updateEngineStatus(false);
                showStatus('Settings saved, but engine not started: ' + error, 'error');
            }
        }
        
        return true;
    } catch (error) {
        console.error('Error saving config:', error);
        showStatus('Error saving settings: ' + error, 'error');
        return false;
    }
}

// Detect PTT key press
let detectingKey = false;
async function detectKeyPress() {
    console.log('detectKeyPress called');
    const btn = document.getElementById('detectKeyBtn');
    const select = document.getElementById('pttKey');
    
    if (detectingKey) {
        console.log('Cancelling detection');
        detectingKey = false;
        btn.textContent = 'Detect Key Press';
        btn.classList.remove('btn-primary');
        btn.classList.add('btn-secondary');
        return;
    }
    
    detectingKey = true;
    btn.textContent = 'Press any key... (Click to cancel)';
    btn.classList.remove('btn-secondary');
    btn.classList.add('btn-primary');
    showStatus('Waiting for key press...', '');
    
    console.log('Calling detect_key_press command...');
    
    try {
        const detectedKey = await invoke('detect_key_press');
        console.log('Detected key:', detectedKey);
        if (detectedKey && detectingKey) {
            select.value = detectedKey;
            showStatus('Detected key: ' + detectedKey, 'success');
        }
    } catch (error) {
        console.error('Error detecting key:', error);
        showStatus('Error detecting key: ' + error, 'error');
    } finally {
        detectingKey = false;
        btn.textContent = 'Detect Key Press';
        btn.classList.remove('btn-primary');
        btn.classList.add('btn-secondary');
    }
}


// Validate API key
async function validateApiKey() {
    try {
        const apiKey = document.getElementById('apiKey').value;
        if (!apiKey) {
            showStatus('Please enter an API key first', 'error');
            return;
        }
        const isValid = await invoke('validate_api_key', { apiKey });
        showStatus(isValid ? 'API key format is valid ✓' : 'Invalid API key format', isValid ? 'success' : 'error');
    } catch (error) {
        showStatus('Error validating API key: ' + error, 'error');
    }
}

// Show status message
function showStatus(message, type = '') {
    const statusEl = document.getElementById('status');
    const statusEl2 = document.getElementById('status2');
    
    [statusEl, statusEl2].forEach(el => {
        if (el) {
            el.textContent = message;
            el.className = 'status ' + type;
        }
    });
    
    setTimeout(() => {
        [statusEl, statusEl2].forEach(el => {
            if (el) {
                el.textContent = '';
                el.className = 'status';
            }
        });
    }, 5000);
}

async function refreshLicenseStatus() {
    try {
        const status = await invoke('fetch_license_status');
        licenseInfo.status = status.status || 'Unknown';
        licenseInfo.plan = status.plan || 'Unknown';
        licenseInfo.key = status.key || null;
        licenseInfo.expiresAt = status.expires_at || null;
        licenseInfo.maxMachines = status.max_machines || null;
        licenseInfo.machinesUsed = status.machines_used || null;
        licenseInfo.hasLicense = true;
        updateLicenseSection();
    } catch (error) {
        console.warn('License status unavailable:', error);
        licenseInfo.status = 'Unlicensed';
        licenseInfo.hasLicense = false;
        updateLicenseSection();
    }
}

async function activateLicense() {
    const keyInput = document.getElementById('licenseKey');
    const licenseKey = keyInput.value.trim();
    const statusEl = document.getElementById('licenseStatus3');
    
    if (!licenseKey) {
        statusEl.textContent = 'Please enter a license key.';
        statusEl.className = 'status error';
        setTimeout(() => {
            statusEl.textContent = '';
            statusEl.className = 'status';
        }, 5000);
        return;
    }

    try {
        statusEl.textContent = 'Activating license...';
        statusEl.className = 'status';
        
        const status = await invoke('activate_license', { licenseKey });
        licenseInfo.status = status.status || 'Active';
        licenseInfo.plan = status.plan || 'Pro';
        licenseInfo.key = status.key || licenseKey;
        licenseInfo.expiresAt = status.expires_at || null;
        licenseInfo.maxMachines = status.max_machines || null;
        licenseInfo.machinesUsed = status.machines_used || null;
        licenseInfo.hasLicense = true;
        updateLicenseSection();
        
        statusEl.textContent = 'License activated successfully! Starting engine...';
        statusEl.className = 'status success';
        keyInput.value = ''; // Clear the input field after success
        
        // Try to start the engine now that we have a valid license
        try {
            await invoke('start_engine');
            const isRunning = await invoke('is_running');
            await updateEngineStatus(isRunning);
            if (isRunning) {
                statusEl.textContent = 'License activated and engine started!';
            } else {
                statusEl.textContent = 'License activated, but engine failed to start. Check settings.';
            }
        } catch (engineError) {
            console.error('Failed to start engine after activation:', engineError);
            await updateEngineStatus(false);
            statusEl.textContent = 'License activated, but engine failed to start: ' + engineError;
        }
        
        setTimeout(() => {
            statusEl.textContent = '';
            statusEl.className = 'status';
        }, 5000);
    } catch (error) {
        console.error('Activation failed:', error);
        // Show more specific error messages
        let errorMsg = '';
        if (error.toString().includes('Invalid license key')) {
            errorMsg = 'Invalid license key';
        } else if (error.toString().includes('not found')) {
            errorMsg = 'License key not found';
        } else if (error.toString().includes('suspended')) {
            errorMsg = 'License suspended';
        } else if (error.toString().includes('expired')) {
            errorMsg = 'License expired';
        } else if (error.toString().includes('max machines') || error.toString().includes('maximum')) {
            errorMsg = 'Maximum devices reached';
        } else if (error.toString().includes('Licensing not configured')) {
            errorMsg = 'Licensing system not configured. Contact support.';
        } else {
            errorMsg = 'Activation failed: ' + error.toString();
        }
        
        statusEl.textContent = errorMsg;
        statusEl.className = 'status error';
        licenseInfo.hasLicense = false;
        updateLicenseSection();
        
        // Keep error visible longer
        setTimeout(() => {
            statusEl.textContent = '';
            statusEl.className = 'status';
        }, 8000);
    }
}

async function updateLicenseSection() {
    const statusEl = document.getElementById('licenseStatus');
    
    // Check trial status
    let trialStatus = null;
    try {
        trialStatus = await invoke('get_trial_status');
    } catch (e) {
        console.warn('Failed to get trial status:', e);
    }
    
    // Determine effective status
    let status = 'unlicensed';
    let displayText = 'Unlicensed';
    
    if (licenseInfo.hasLicense) {
        status = (licenseInfo.status || 'unknown').toLowerCase();
        displayText = licenseInfo.status || 'Unknown';
    } else if (trialStatus && trialStatus.is_trial) {
        if (trialStatus.expired) {
            status = 'expired';
            displayText = 'Trial Expired';
        } else {
            status = 'trial';
            displayText = 'Trial Active';
        }
    }
    
    // Clear and create badge
    statusEl.innerHTML = '';
    const badge = document.createElement('span');
    badge.className = `status-badge ${status}`;
    badge.textContent = displayText;
    statusEl.appendChild(badge);
    
    // Update plan
    if (trialStatus && trialStatus.is_trial && !licenseInfo.hasLicense) {
        document.getElementById('licensePlan').textContent = 'Trial';
    } else {
        document.getElementById('licensePlan').textContent = licenseInfo.plan || '—';
    }
    
    // Show trial time remaining if in trial
    const trialDaysRow = document.getElementById('trialDaysRow');
    const trialDaysEl = document.getElementById('trialDaysRemaining');
    if (trialStatus && trialStatus.is_trial && !licenseInfo.hasLicense && trialStatus.expiration_date) {
        // Start live countdown
        startTrialCountdown(trialStatus.expiration_date, trialDaysEl, trialStatus.human_remaining);
        trialDaysRow.style.display = 'block';
    } else {
        // Stop countdown if it was running
        stopTrialCountdown();
        trialDaysRow.style.display = 'none';
    }

    const keyRow = document.getElementById('licenseKeyRow');
    const keyDisplay = document.getElementById('licenseKeyDisplay');
    if (licenseInfo.key && licenseInfo.hasLicense) {
        // Show masked key by default (only first 6 characters visible)
        if (!window.licenseKeyVisible) {
            const maskedKey = licenseInfo.key.slice(0, 6) + '•'.repeat(Math.max(0, licenseInfo.key.length - 6));
            keyDisplay.textContent = maskedKey;
        } else {
            keyDisplay.textContent = licenseInfo.key;
        }
        keyRow.style.display = 'block';
    } else {
        keyRow.style.display = 'none';
    }

    const expiresEl = document.getElementById('licenseExpires');
    if (licenseInfo.expiresAt) {
        const date = new Date(licenseInfo.expiresAt);
        expiresEl.textContent = date.toLocaleString();
    } else if (trialStatus && trialStatus.is_trial && trialStatus.expiration_date) {
        const date = new Date(trialStatus.expiration_date);
        expiresEl.textContent = date.toLocaleString();
    } else if (licenseInfo.hasLicense) {
        // Has license but no expiration date = never expires
        expiresEl.textContent = 'Never';
    } else {
        expiresEl.textContent = '—';
    }

    const devicesEl = document.getElementById('licenseDevices');
    if (licenseInfo.maxMachines != null && licenseInfo.machinesUsed != null) {
        devicesEl.textContent = `${licenseInfo.machinesUsed} / ${licenseInfo.maxMachines}`;
    } else {
        devicesEl.textContent = '—';
    }

    const message = document.getElementById('licenseMessage');
    const trialSection = document.getElementById('trialSection');
    const activateSection = document.getElementById('activateSection');
    const buySection = document.getElementById('buySection');
    const deactivateSection = document.getElementById('deactivateSection');
    
    // Show/hide sections based on license/trial status
    if (licenseInfo.hasLicense) {
        // Has active license
        message.textContent = 'License active. Thank you for supporting DeskTalk!';
        message.classList.remove('error');
        message.classList.add('success');
        trialSection.style.display = 'none';
        activateSection.style.display = 'none';
        buySection.style.display = 'none';
        deactivateSection.style.display = 'block';
    } else if (trialStatus && trialStatus.is_trial) {
        // In trial (active or expired)
        if (trialStatus.expired) {
            message.textContent = 'Your trial has expired. Purchase a license to continue using DeskTalk.';
            message.classList.remove('success');
            message.classList.add('error');
        } else {
            const remainingText = trialStatus.human_remaining || 'Time remaining unknown';
            message.textContent = `Trial active! ${remainingText} remaining. Purchase a license for unlimited access.`;
            message.classList.remove('error');
            message.classList.add('success');
        }
        trialSection.style.display = 'none';
        activateSection.style.display = 'block';
        buySection.style.display = 'block';
        deactivateSection.style.display = 'none';
    } else {
        // No license, no trial
        message.textContent = 'Start a free 7-day trial or enter your license key.';
        message.classList.remove('success');
        message.classList.add('error');
        trialSection.style.display = 'block';
        activateSection.style.display = 'block';
        buySection.style.display = 'block';
        deactivateSection.style.display = 'none';
    }
}

async function startTrial() {
    const statusEl = document.getElementById('trialStatus');
    const startBtn = document.getElementById('startTrialBtn');
    
    try {
        statusEl.textContent = 'Starting trial...';
        statusEl.className = 'status';
        startBtn.disabled = true;
        
        const result = await invoke('start_trial');
        
        statusEl.textContent = 'Trial started! You have 7 days of full access.';
        statusEl.className = 'status success';
        
        // Refresh license section to show trial status
        await updateLicenseSection();
        
        // Try to start the engine now that trial is active
        try {
            await invoke('start_engine');
            await updateEngineStatus(true);
        } catch (e) {
            console.warn('Failed to auto-start engine after trial:', e);
        }
        
        setTimeout(() => {
            statusEl.textContent = '';
            statusEl.className = 'status';
        }, 5000);
    } catch (error) {
        console.error('Failed to start trial:', error);
        statusEl.textContent = 'Failed to start trial: ' + error;
        statusEl.className = 'status error';
        startBtn.disabled = false;
        
        setTimeout(() => {
            statusEl.textContent = '';
            statusEl.className = 'status';
        }, 5000);
    }
}

// Dark mode toggle
document.getElementById('darkMode').addEventListener('change', (e) => {
    if (e.target.checked) {
        document.body.classList.add('dark-mode');
    } else {
        document.body.classList.remove('dark-mode');
    }
});

// Event listeners
document.getElementById('saveBtn').addEventListener('click', saveConfig);
document.getElementById('saveBtn2').addEventListener('click', saveConfig);
document.getElementById('validateKeyBtn').addEventListener('click', validateApiKey);
document.getElementById('refreshDevicesBtn').addEventListener('click', loadAudioDevices);
document.getElementById('activateLicenseBtn').addEventListener('click', activateLicense);

// Only add listener if button exists (it's commented out in HTML)
const detectKeyBtn = document.getElementById('detectKeyBtn');
if (detectKeyBtn) {
    detectKeyBtn.addEventListener('click', detectKeyPress);
}

document.getElementById('startTrialBtn').addEventListener('click', startTrial);

document.getElementById('buyLicenseBtn').addEventListener('click', async () => {
    try {
        await invoke('open_url', { url: 'https://buy.stripe.com/5kQcMY0s96f051Lg7U0sU01' });
    } catch (error) {
        console.error('Failed to open purchase page:', error);
        showStatus('Failed to open purchase page', 'error');
    }
});

document.getElementById('viewUsageBtn').addEventListener('click', async () => {
    try {
        await invoke('open_url', { url: 'https://platform.openai.com/usage' });
    } catch (error) {
        console.error('Failed to open URL:', error);
        showStatus('Failed to open browser: ' + error, 'error');
    }
});

document.getElementById('openaiApiKeysLink').addEventListener('click', async (e) => {
    e.preventDefault();
    try {
        await invoke('open_url', { url: 'https://platform.openai.com/api-keys' });
    } catch (error) {
        console.error('Failed to open URL:', error);
        showStatus('Failed to open browser: ' + error, 'error');
    }
});

document.getElementById('apiKeyVideoLink').addEventListener('click', async (e) => {
    e.preventDefault();
    try {
        await invoke('open_url', { url: 'https://youtu.be/SzPE_AE0eEo?si=WbJP-ABj0uG5s-XV' });
    } catch (error) {
        console.error('Failed to open URL:', error);
        showStatus('Failed to open browser: ' + error, 'error');
    }
});

document.getElementById('deactivateLicenseBtn').addEventListener('click', async () => {
    if (!confirm('Are you sure you want to deactivate this license from this device?')) {
        return;
    }
    try {
        // Stop the engine first (license is required)
        const wasRunning = await invoke('is_running');
        if (wasRunning) {
            await invoke('stop_engine');
        }
        
        await invoke('deactivate_license');
        licenseInfo.hasLicense = false;
        licenseInfo.status = 'Unlicensed';
        licenseInfo.plan = 'Unknown';
        licenseInfo.key = null;
        licenseInfo.expiresAt = null;
        licenseInfo.maxMachines = null;
        licenseInfo.machinesUsed = null;
        updateLicenseSection();
        
        // Check if trial is active, and if so, restart the engine
        let trialStatus = null;
        try {
            trialStatus = await invoke('get_trial_status');
        } catch (e) {
            console.warn('Failed to check trial status:', e);
        }
        
        if (trialStatus && trialStatus.is_trial && !trialStatus.expired) {
            // Trial is active, try to restart engine
            console.log('Trial is active after deactivation, attempting to restart engine...');
            try {
                await invoke('start_engine');
                await updateEngineStatus(true);
                showStatus('License deactivated. Trial is active - engine restarted.', 'success');
            } catch (e) {
                console.error('Failed to restart engine with trial:', e);
                await updateEngineStatus(false);
                showStatus('License deactivated. Engine stopped: ' + e, 'error');
            }
        } else {
            // No active trial
            await updateEngineStatus(false);
            showStatus('License deactivated successfully! Engine stopped.', 'success');
        }
    } catch (error) {
        console.error('Failed to deactivate license:', error);
        showStatus('Failed to deactivate license: ' + error, 'error');
    }
});

// Initialize license key visibility state
window.licenseKeyVisible = false;

// Initialize
console.log('App.js loaded, initializing...');
document.addEventListener('DOMContentLoaded', () => {
    const apiKeyInput = document.getElementById('apiKey');
    if (apiKeyInput) {
        apiKeyInput.addEventListener('input', (event) => {
            cachedApiKey = event.target.value;
        }, { passive: true });
    }

    // License key toggle button
    const toggleKeyBtn = document.getElementById('toggleKeyBtn');
    if (toggleKeyBtn) {
        toggleKeyBtn.addEventListener('click', () => {
            window.licenseKeyVisible = !window.licenseKeyVisible;
            toggleKeyBtn.textContent = window.licenseKeyVisible ? '🙈' : '👁️';
            toggleKeyBtn.title = window.licenseKeyVisible ? 'Hide Key' : 'Show Key';
            updateLicenseSection();
        });
    }

    // License key copy button
    const copyKeyBtn = document.getElementById('copyKeyBtn');
    if (copyKeyBtn) {
        copyKeyBtn.addEventListener('click', async () => {
            if (licenseInfo.key) {
                try {
                    await navigator.clipboard.writeText(licenseInfo.key);
                    const originalText = copyKeyBtn.textContent;
                    copyKeyBtn.textContent = '✅';
                    setTimeout(() => {
                        copyKeyBtn.textContent = originalText;
                    }, 2000);
                } catch (err) {
                    console.error('Failed to copy license key:', err);
                    copyKeyBtn.textContent = '❌';
                    setTimeout(() => {
                        copyKeyBtn.textContent = '📋';
                    }, 2000);
                }
            }
        });
    }

    (async () => {
        await loadPTTKeys();
        await loadConfig();
        await loadAudioDevices();
        console.log('Initialization complete');
    })();

setInterval(() => {
    const statsTab = document.getElementById('stats');
    if (statsTab && statsTab.classList.contains('active')) {
        loadStatistics();
    }
}, 1500);

// Load license status once when switching to License tab (not repeatedly)
document.querySelectorAll('.tab-button').forEach(button => {
    button.addEventListener('click', () => {
        if (button.dataset.tab === 'license') {
            refreshLicenseStatus();
        }
    });
});
});