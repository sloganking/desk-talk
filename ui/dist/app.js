// Get the invoke function from Tauri API (supports both v1 and v2)
const invoke = window.__TAURI_INTERNALS__?.invoke || window.__TAURI__?.core?.invoke || window.__TAURI__?.tauri?.invoke;

if (!invoke) {
    console.error('Tauri invoke function not found!');
    alert('ERROR: Tauri API not loaded. The app may not work correctly.');
}

let cachedApiKey = '';
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
        document.getElementById('punctuation').checked = config.punctuation || false;
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
        
        // Typing speed
        if (config.typing_wpm) {
            document.getElementById('typingWPM').value = config.typing_wpm;
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
        } else {
            console.warn('✗ No API key in loaded config');
            apiKeyField.value = ''; // Clear the field
        }

        if (config.local_model) {
            document.getElementById('localModel').value = config.local_model;
        }

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

// Format time duration
function formatDuration(totalSeconds) {
    const hours = Math.floor(totalSeconds / 3600);
    const minutes = Math.floor((totalSeconds % 3600) / 60);
    const seconds = Math.floor(totalSeconds % 60);
    
    if (hours > 0) {
        return `${hours}h ${minutes}m`;
    } else if (minutes > 0) {
        return `${minutes}m ${seconds}s`;
    } else {
        return `${seconds}s`;
    }
}

// Format large numbers with commas
function formatNumber(num) {
    return num.toLocaleString();
}

// Load statistics
async function loadStatistics() {
    try {
        const stats = await invoke('get_statistics');
        
        const typingWPM = stats.typing_wpm || 40;
        
        // Session stats
        document.getElementById('totalWords').textContent = formatNumber(stats.total_words || 0);
        document.getElementById('avgWPM').textContent = (stats.average_wpm || 0).toFixed(1);
        document.getElementById('totalTime').textContent = formatDuration(stats.total_recording_time_secs || 0);
        document.getElementById('sessionCount').textContent = formatNumber(stats.session_count || 0);
        document.getElementById('timeSaved').textContent = formatDuration(stats.time_saved_secs || 0);
        
        // Session speed multiplier (only show if session has data)
        const sessionWPM = stats.average_wpm || 0;
        const speedMultiplierEl = document.getElementById('speedMultiplier');
        if (sessionWPM > 0 && typingWPM > 0) {
            const multiplier = sessionWPM / typingWPM;
            speedMultiplierEl.textContent = `${multiplier.toFixed(1)}x`;
            speedMultiplierEl.title = `You speak ${multiplier.toFixed(1)}x faster than you type`;
        } else {
            speedMultiplierEl.textContent = '';
        }
        
        // Session efficiency percentage
        const timeSavedPercentEl = document.getElementById('timeSavedPercent');
        if (stats.total_words > 0 && typingWPM > 0) {
            const timeToType = (stats.total_words * 60) / typingWPM;
            const efficiency = (stats.time_saved_secs / timeToType) * 100;
            timeSavedPercentEl.textContent = `${efficiency.toFixed(0)}% saved`;
        } else {
            timeSavedPercentEl.textContent = '';
        }
        
        // Lifetime stats
        document.getElementById('lifetimeWords').textContent = `Lifetime: ${formatNumber(stats.lifetime_total_words || 0)}`;
        document.getElementById('lifetimeWPM').textContent = `Lifetime: ${(stats.lifetime_average_wpm || 0).toFixed(1)}`;
        document.getElementById('lifetimeTime').textContent = `Lifetime: ${formatDuration(stats.lifetime_total_recording_time_secs || 0)}`;
        document.getElementById('lifetimeSessions').textContent = `Lifetime: ${formatNumber(stats.lifetime_session_count || 0)}`;
        document.getElementById('lifetimeTimeSaved').textContent = `Lifetime: ${formatDuration(stats.lifetime_time_saved_secs || 0)}`;
        
        // Lifetime speed multiplier
        const lifetimeWPM = stats.lifetime_average_wpm || 0;
        const lifetimeSpeedMultiplierEl = document.getElementById('lifetimeSpeedMultiplier');
        if (lifetimeWPM > 0 && typingWPM > 0) {
            const multiplier = lifetimeWPM / typingWPM;
            lifetimeSpeedMultiplierEl.textContent = `${multiplier.toFixed(1)}x`;
            lifetimeSpeedMultiplierEl.title = `You speak ${multiplier.toFixed(1)}x faster than you type`;
        } else {
            lifetimeSpeedMultiplierEl.textContent = '';
        }
        
        // Lifetime efficiency percentage
        const lifetimeTimeSavedPercentEl = document.getElementById('lifetimeTimeSavedPercent');
        if (stats.lifetime_total_words > 0 && typingWPM > 0) {
            const timeToType = (stats.lifetime_total_words * 60) / typingWPM;
            const efficiency = (stats.lifetime_time_saved_secs / timeToType) * 100;
            lifetimeTimeSavedPercentEl.textContent = `${efficiency.toFixed(0)}% saved`;
        } else {
            lifetimeTimeSavedPercentEl.textContent = '';
        }
        
        // Update typing WPM input if it hasn't been touched
        const typingInput = document.getElementById('typingWPM');
        if (typingInput && !typingInput.dataset.userEdited) {
            typingInput.value = stats.typing_wpm || 40;
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
                punctuation: document.getElementById('punctuation').checked,
                type_chars: document.getElementById('typeChars').checked,
                auto_start: document.getElementById('autoStart').checked,
                start_minimized: document.getElementById('startMinimized').checked,
                dark_mode: document.getElementById('darkMode').checked,
                api_key: document.getElementById('apiKey').value || cachedApiKey || null,
                typing_wpm: parseInt(document.getElementById('typingWPM').value) || 40,
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
            punctuation: document.getElementById('punctuation').checked,
            type_chars: document.getElementById('typeChars').checked,
            auto_start: document.getElementById('autoStart').checked,
            start_minimized: document.getElementById('startMinimized').checked,
            dark_mode: document.getElementById('darkMode').checked,
            api_key: apiKey || null,
            typing_wpm: parseInt(document.getElementById('typingWPM').value) || 40,
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

// Only add listener if button exists (it's commented out in HTML)
const detectKeyBtn = document.getElementById('detectKeyBtn');
if (detectKeyBtn) {
    detectKeyBtn.addEventListener('click', detectKeyPress);
}

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

// Typing test link
document.getElementById('typingTestLink').addEventListener('click', async (e) => {
    e.preventDefault();
    try {
        await invoke('open_url', { url: 'https://www.octotyping.com/typing-speed-test' });
    } catch (error) {
        console.error('Failed to open URL:', error);
        showStatus('Failed to open browser: ' + error, 'error');
    }
});

// Auto-save typing WPM when changed
let typingWPMSaveTimeout = null;
document.getElementById('typingWPM').addEventListener('input', (e) => {
    e.target.dataset.userEdited = 'true';
    
    // Debounce the save
    if (typingWPMSaveTimeout) {
        clearTimeout(typingWPMSaveTimeout);
    }
    typingWPMSaveTimeout = setTimeout(async () => {
        try {
            const config = await invoke('get_config');
            config.typing_wpm = parseInt(e.target.value) || 40;
            await invoke('save_config', { incoming: config });
            console.log('Typing WPM saved:', config.typing_wpm);
            // Refresh stats to show updated time saved
            loadStatistics();
        } catch (error) {
            console.error('Failed to save typing WPM:', error);
        }
    }, 500);
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
});

// NOTE: PTT doesn't work when DeskTalk window is focused - this is a Windows limitation
// ALL global hotkey apps have this same behavior (Discord, OBS, etc.)
// Solution: Minimize DeskTalk after starting transcription, use PTT in other apps
// The settings window is just for configuration, not active use
