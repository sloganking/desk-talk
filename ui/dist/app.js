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

function updateEngineButtons(isRunning) {
    const buttons = [
        { start: document.getElementById('startBtn'), stop: document.getElementById('stopBtn') },
        { start: document.getElementById('startBtn2'), stop: document.getElementById('stopBtn2') }
    ];
    
    buttons.forEach(btnSet => {
        if (btnSet.start && btnSet.stop) {
            if (isRunning) {
                btnSet.start.style.display = 'none';
                btnSet.stop.style.display = 'inline-block';
            } else {
                btnSet.start.style.display = 'inline-block';
                btnSet.stop.style.display = 'none';
            }
        }
    });
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
        updateEngineButtons(running);
        
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
        
        // Validate required fields
        if (!pttKeyValue) {
            showStatus('Please select a PTT key first!', 'error');
            return false;
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
        
        await invoke('save_config', { incoming: config });
        showStatus('Settings saved successfully!', 'success');
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

// Start transcription
async function startTranscription() {
    try {
        // Save and validate config first
        const saved = await saveConfig();
        if (!saved) {
            return;
        }
        
        await invoke('start_engine');
        document.getElementById('startBtn').style.display = 'none';
        document.getElementById('stopBtn').style.display = 'inline-block';
        showStatus('Transcription started! Hold your PTT key to record.', 'success');
    } catch (error) {
        showStatus('Error starting transcription: ' + error, 'error');
    }
}

// Stop transcription
async function stopTranscription() {
    try {
        await invoke('stop_engine');
        document.getElementById('startBtn').style.display = 'inline-block';
        document.getElementById('stopBtn').style.display = 'none';
        showStatus('Transcription stopped', 'success');
    } catch (error) {
        showStatus('Error stopping transcription: ' + error, 'error');
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
    if (!licenseKey) {
        showStatus('Please enter a license key.', 'error');
        return;
    }

    try {
        showStatus('Activating license...', '');
        const status = await invoke('activate_license', { licenseKey });
        licenseInfo.status = status.status || 'Active';
        licenseInfo.plan = status.plan || 'Pro';
        licenseInfo.expiresAt = status.expires_at || null;
        licenseInfo.maxMachines = status.max_machines || null;
        licenseInfo.machinesUsed = status.machines_used || null;
        licenseInfo.hasLicense = true;
        updateLicenseSection();
        showStatus('License activated successfully!', 'success');
    } catch (error) {
        console.error('Activation failed:', error);
        showStatus('Activation failed: ' + error, 'error');
        licenseInfo.hasLicense = false;
        updateLicenseSection();
    }
}

function updateLicenseSection() {
    const statusEl = document.getElementById('licenseStatus');
    const status = (licenseInfo.status || 'Unknown').toLowerCase();
    
    // Clear and create badge
    statusEl.innerHTML = '';
    const badge = document.createElement('span');
    badge.className = `status-badge ${status}`;
    badge.textContent = licenseInfo.status || 'Unknown';
    statusEl.appendChild(badge);
    
    document.getElementById('licensePlan').textContent = licenseInfo.plan || 'Unknown';

    const keyRow = document.getElementById('licenseKeyRow');
    const keyDisplay = document.getElementById('licenseKeyDisplay');
    if (licenseInfo.key && licenseInfo.hasLicense) {
        keyDisplay.textContent = licenseInfo.key;
        keyRow.style.display = 'block';
    } else {
        keyRow.style.display = 'none';
    }

    const expiresEl = document.getElementById('licenseExpires');
    if (licenseInfo.expiresAt) {
        const date = new Date(licenseInfo.expiresAt);
        expiresEl.textContent = date.toLocaleString();
    } else {
        expiresEl.textContent = '—';
    }

    const devicesEl = document.getElementById('licenseDevices');
    if (licenseInfo.maxMachines != null) {
        const used = licenseInfo.machinesUsed != null ? licenseInfo.machinesUsed : '?';
        devicesEl.textContent = `${used} / ${licenseInfo.maxMachines}`;
    } else {
        devicesEl.textContent = '—';
    }

    const message = document.getElementById('licenseMessage');
    const activateSection = document.querySelector('#license .section:nth-child(2)'); // Activate License section
    const buySection = document.querySelector('#license .section:nth-child(3)'); // Buy License section
    const deactivateSection = document.getElementById('deactivateSection');
    
    if (!licenseInfo.hasLicense) {
        message.textContent = 'Enter your license key to unlock DeskTalk Pro features.';
        message.classList.remove('success');
        message.classList.add('error');
        if (activateSection) activateSection.style.display = 'block';
        if (buySection) buySection.style.display = 'block';
        if (deactivateSection) deactivateSection.style.display = 'none';
    } else if (licenseInfo.status && licenseInfo.status.toLowerCase() === 'suspended') {
        message.textContent = 'License suspended. Contact support to restore access.';
        message.classList.remove('success');
        message.classList.add('error');
        if (activateSection) activateSection.style.display = 'none';
        if (buySection) buySection.style.display = 'block';
        if (deactivateSection) deactivateSection.style.display = 'block';
    } else {
        message.textContent = 'License active. Thank you for supporting DeskTalk!';
        message.classList.remove('error');
        message.classList.add('success');
        if (activateSection) activateSection.style.display = 'none';
        if (buySection) buySection.style.display = 'none';
        if (deactivateSection) deactivateSection.style.display = 'block';
    }
}

// Event listeners
document.getElementById('saveBtn').addEventListener('click', saveConfig);
document.getElementById('saveBtn2').addEventListener('click', saveConfig);
document.getElementById('startBtn').addEventListener('click', startTranscription);
document.getElementById('startBtn2').addEventListener('click', startTranscription);
document.getElementById('stopBtn').addEventListener('click', stopTranscription);
document.getElementById('stopBtn2').addEventListener('click', stopTranscription);
document.getElementById('validateKeyBtn').addEventListener('click', validateApiKey);
document.getElementById('refreshDevicesBtn').addEventListener('click', loadAudioDevices);
document.getElementById('activateLicenseBtn').addEventListener('click', activateLicense);

// Only add listener if button exists (it's commented out in HTML)
const detectKeyBtn = document.getElementById('detectKeyBtn');
if (detectKeyBtn) {
    detectKeyBtn.addEventListener('click', detectKeyPress);
}

document.getElementById('buyLicenseBtn').addEventListener('click', async () => {
    try {
        // TODO: Replace with actual purchase URL when set up
        await invoke('open_url', { url: 'https://example.com/buy-desktalk' });
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

document.getElementById('deactivateLicenseBtn').addEventListener('click', async () => {
    if (!confirm('Are you sure you want to deactivate this license from this device?')) {
        return;
    }
    try {
        await invoke('deactivate_license');
        licenseInfo.hasLicense = false;
        licenseInfo.status = 'Unlicensed';
        licenseInfo.plan = 'Unknown';
        licenseInfo.key = null;
        licenseInfo.expiresAt = null;
        licenseInfo.maxMachines = null;
        licenseInfo.machinesUsed = null;
        updateLicenseSection();
        showStatus('License deactivated successfully!', 'success');
    } catch (error) {
        console.error('Failed to deactivate license:', error);
        showStatus('Failed to deactivate license: ' + error, 'error');
    }
});

// Initialize
console.log('App.js loaded, initializing...');
document.addEventListener('DOMContentLoaded', () => {
    const apiKeyInput = document.getElementById('apiKey');
    if (apiKeyInput) {
        apiKeyInput.addEventListener('input', (event) => {
            cachedApiKey = event.target.value;
        }, { passive: true });
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