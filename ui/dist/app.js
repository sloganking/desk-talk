// Get the invoke function from Tauri API (supports both v1 and v2)
const invoke = window.__TAURI_INTERNALS__?.invoke || window.__TAURI__?.core?.invoke || window.__TAURI__?.tauri?.invoke;

if (!invoke) {
    console.error('Tauri invoke function not found!');
    alert('ERROR: Tauri API not loaded. The app may not work correctly.');
}

let cachedApiKey = '';

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
        
        // General settings
        if (config.ptt_key) {
            document.getElementById('pttKey').value = config.ptt_key;
        }
        document.getElementById('audioDevice').value = config.device || 'default';
        document.getElementById('capFirst').checked = config.cap_first || false;
        document.getElementById('space').checked = config.space || false;
        document.getElementById('typeChars').checked = config.type_chars || false;
        document.getElementById('autoStart').checked = config.auto_start || false;
        
        // Transcription settings
        const isLocal = config.use_local || false;
        document.getElementById('modeOpenAI').checked = !isLocal;
        document.getElementById('modeLocal').checked = isLocal;
        document.getElementById('openaiSection').style.display = isLocal ? 'none' : 'block';
        document.getElementById('localSection').style.display = isLocal ? 'block' : 'none';
        
        if (config.api_key) {
            cachedApiKey = config.api_key;
            document.getElementById('apiKey').value = config.api_key;
        }
        if (config.local_model) {
            document.getElementById('localModel').value = config.local_model;
        }
        
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
        
        const totalMins = Math.floor((stats.total_recording_time_secs || 0) / 60);
        document.getElementById('totalTime').textContent = totalMins + 'm';
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
            api_key: apiKey || null,
        };
        
        console.log('Config payload being sent:', JSON.stringify({ ...config, api_key: apiKey ? '(hidden)' : null }, null, 2));
        
        await invoke('save_config', { config });
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
    statusEl.textContent = message;
    statusEl.className = 'status ' + type;
    setTimeout(() => {
        statusEl.textContent = '';
        statusEl.className = 'status';
    }, 5000);
}

// Event listeners
document.getElementById('saveBtn').addEventListener('click', saveConfig);
document.getElementById('startBtn').addEventListener('click', startTranscription);
document.getElementById('stopBtn').addEventListener('click', stopTranscription);
document.getElementById('validateKeyBtn').addEventListener('click', validateApiKey);
document.getElementById('refreshDevicesBtn').addEventListener('click', loadAudioDevices);

// Only add listener if button exists (it's commented out in HTML)
const detectKeyBtn = document.getElementById('detectKeyBtn');
if (detectKeyBtn) {
    detectKeyBtn.addEventListener('click', detectKeyPress);
}

document.getElementById('buyLicenseBtn').addEventListener('click', () => {
    // Will be replaced with actual purchase URL
    window.open('https://example.com/buy', '_blank');
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

    loadConfig();
    loadPTTKeys();
    loadAudioDevices();
    console.log('Initialization complete');
});