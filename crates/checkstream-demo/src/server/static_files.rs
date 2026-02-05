use axum::{
    http::{header, StatusCode, Uri},
    response::{Html, IntoResponse, Response},
};
use rust_embed::Embed;

#[derive(Embed)]
#[folder = "web/dist"]
struct WebAssets;

/// Serve embedded static files from the Vue.js build
pub async fn serve_static(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');

    // Try exact path first
    if let Some(content) = <WebAssets as Embed>::get(path) {
        let mime = mime_guess::from_path(path).first_or_octet_stream();
        return (
            StatusCode::OK,
            [(header::CONTENT_TYPE, mime.as_ref())],
            content.data.into_owned(),
        )
            .into_response();
    }

    // For SPA routing, serve index.html for any unmatched route
    if let Some(content) = <WebAssets as Embed>::get("index.html") {
        return Html(String::from_utf8_lossy(&content.data).to_string()).into_response();
    }

    // Fallback: return a simple HTML page if no frontend is built yet
    Html(FALLBACK_HTML.to_string()).into_response()
}

const FALLBACK_HTML: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>CheckStream Demo</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <style>
        @keyframes pulse { 0%, 100% { opacity: 1; } 50% { opacity: 0.5; } }
        .animate-pulse { animation: pulse 2s cubic-bezier(0.4, 0, 0.6, 1) infinite; }
    </style>
</head>
<body class="bg-gray-900 text-white min-h-screen">
    <div class="container mx-auto px-4 py-8">
        <header class="mb-8">
            <h1 class="text-4xl font-bold text-blue-400">CheckStream Demo</h1>
            <p class="text-gray-400 mt-2">Interactive guardrail demonstration</p>
        </header>

        <div class="grid grid-cols-1 lg:grid-cols-3 gap-6">
            <!-- Stats Panel -->
            <div class="bg-gray-800 rounded-lg p-6">
                <h2 class="text-xl font-semibold mb-4">Live Stats</h2>
                <div id="stats" class="space-y-4">
                    <div class="flex justify-between">
                        <span class="text-gray-400">Requests/min</span>
                        <span id="rpm" class="font-mono text-green-400">0</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-400">Block Rate</span>
                        <span id="block-rate" class="font-mono text-red-400">0%</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-400">Avg Latency</span>
                        <span id="latency" class="font-mono text-yellow-400">0ms</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-400">Total Requests</span>
                        <span id="total" class="font-mono">0</span>
                    </div>
                </div>
            </div>

            <!-- Traffic Controls -->
            <div class="bg-gray-800 rounded-lg p-6">
                <h2 class="text-xl font-semibold mb-4">Traffic Generator</h2>
                <div class="space-y-4">
                    <div>
                        <label class="block text-sm text-gray-400 mb-2">Rate (req/s)</label>
                        <input type="range" id="rate" min="1" max="50" value="10" class="w-full">
                        <span id="rate-value" class="text-sm text-gray-400">10 req/s</span>
                    </div>
                    <div class="space-y-2">
                        <label class="flex items-center space-x-2">
                            <input type="checkbox" id="pii" checked class="rounded">
                            <span>Inject PII</span>
                        </label>
                        <label class="flex items-center space-x-2">
                            <input type="checkbox" id="toxicity" checked class="rounded">
                            <span>Inject Toxicity</span>
                        </label>
                        <label class="flex items-center space-x-2">
                            <input type="checkbox" id="injection" class="rounded">
                            <span>Inject Prompt Injection</span>
                        </label>
                    </div>
                    <button id="toggle-btn" class="w-full bg-green-600 hover:bg-green-700 text-white font-bold py-2 px-4 rounded">
                        Start Traffic
                    </button>
                </div>
            </div>

            <!-- Issue Config -->
            <div class="bg-gray-800 rounded-lg p-6">
                <h2 class="text-xl font-semibold mb-4">Issue Breakdown</h2>
                <div id="issues" class="space-y-2">
                    <div class="flex justify-between">
                        <span class="text-gray-400">PII Detected</span>
                        <span id="pii-count" class="font-mono text-orange-400">0</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-400">Toxicity</span>
                        <span id="toxicity-count" class="font-mono text-red-400">0</span>
                    </div>
                    <div class="flex justify-between">
                        <span class="text-gray-400">Prompt Injection</span>
                        <span id="injection-count" class="font-mono text-purple-400">0</span>
                    </div>
                </div>
            </div>
        </div>

        <!-- Event Log -->
        <div class="mt-6 bg-gray-800 rounded-lg p-6">
            <h2 class="text-xl font-semibold mb-4">Event Log</h2>
            <div id="log" class="h-96 overflow-y-auto font-mono text-sm space-y-1">
                <div class="text-gray-500">Waiting for events...</div>
            </div>
        </div>
    </div>

    <script>
        const ws = new WebSocket(`ws://${window.location.host}/ws`);
        let isRunning = false;
        let stats = { rpm: 0, blockRate: 0, latency: 0, total: 0 };
        let issues = { pii: 0, toxicity: 0, injection: 0 };

        ws.onopen = () => {
            console.log('WebSocket connected');
            addLog('system', 'Connected to CheckStream Demo');
        };

        ws.onmessage = (event) => {
            const data = JSON.parse(event.data);
            handleEvent(data);
        };

        ws.onclose = () => {
            addLog('error', 'WebSocket disconnected');
        };

        function handleEvent(event) {
            if (event.type === 'RequestCompleted') {
                const result = event.data;
                stats.total++;

                // Update log
                const actionColor = {
                    pass: 'text-green-400',
                    block: 'text-red-400',
                    redact: 'text-yellow-400'
                }[result.action] || 'text-gray-400';

                const issueText = result.issues_detected.length > 0
                    ? ` [${result.issues_detected.map(i => i.issue_type).join(', ')}]`
                    : '';

                addLog(result.action, `${result.action.toUpperCase()} ${result.latency_ms.toFixed(1)}ms${issueText}`);

                // Update issue counts
                result.issues_detected.forEach(issue => {
                    if (issue.issue_type === 'ssn' || issue.issue_type === 'credit_card' || issue.issue_type === 'email') {
                        issues.pii++;
                    } else if (issue.issue_type === 'toxicity') {
                        issues.toxicity++;
                    } else if (issue.issue_type === 'prompt_injection') {
                        issues.injection++;
                    }
                });
                updateIssues();
            } else if (event.type === 'MetricsUpdate') {
                stats.rpm = event.data.requests_per_minute;
                stats.blockRate = event.data.block_rate * 100;
                stats.latency = event.data.avg_latency_ms;
                updateStats();
            } else if (event.type === 'TrafficStateChanged') {
                isRunning = event.data === 'running';
                updateButton();
            }
        }

        function addLog(type, message) {
            const log = document.getElementById('log');
            const colors = {
                pass: 'text-green-400',
                block: 'text-red-400',
                redact: 'text-yellow-400',
                system: 'text-blue-400',
                error: 'text-red-500'
            };
            const color = colors[type] || 'text-gray-400';
            const time = new Date().toLocaleTimeString();
            const entry = document.createElement('div');
            entry.className = color;
            entry.textContent = `[${time}] ${message}`;

            // Remove "waiting" message
            if (log.children.length === 1 && log.children[0].textContent.includes('Waiting')) {
                log.innerHTML = '';
            }

            log.insertBefore(entry, log.firstChild);
            if (log.children.length > 100) {
                log.removeChild(log.lastChild);
            }
        }

        function updateStats() {
            document.getElementById('rpm').textContent = stats.rpm.toFixed(1);
            document.getElementById('block-rate').textContent = stats.blockRate.toFixed(1) + '%';
            document.getElementById('latency').textContent = stats.latency.toFixed(1) + 'ms';
            document.getElementById('total').textContent = stats.total;
        }

        function updateIssues() {
            document.getElementById('pii-count').textContent = issues.pii;
            document.getElementById('toxicity-count').textContent = issues.toxicity;
            document.getElementById('injection-count').textContent = issues.injection;
        }

        function updateButton() {
            const btn = document.getElementById('toggle-btn');
            if (isRunning) {
                btn.textContent = 'Stop Traffic';
                btn.className = 'w-full bg-red-600 hover:bg-red-700 text-white font-bold py-2 px-4 rounded';
            } else {
                btn.textContent = 'Start Traffic';
                btn.className = 'w-full bg-green-600 hover:bg-green-700 text-white font-bold py-2 px-4 rounded';
            }
        }

        // Rate slider
        document.getElementById('rate').addEventListener('input', (e) => {
            document.getElementById('rate-value').textContent = e.target.value + ' req/s';
        });

        // Toggle button
        document.getElementById('toggle-btn').addEventListener('click', async () => {
            const endpoint = isRunning ? '/api/traffic/stop' : '/api/traffic/start';
            const body = isRunning ? {} : {
                rate: parseInt(document.getElementById('rate').value),
                issue_config: {
                    pii_enabled: document.getElementById('pii').checked,
                    toxicity_enabled: document.getElementById('toxicity').checked,
                    injection_enabled: document.getElementById('injection').checked
                }
            };

            try {
                await fetch(endpoint, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify(body)
                });
            } catch (err) {
                addLog('error', 'Failed to toggle traffic: ' + err.message);
            }
        });
    </script>
</body>
</html>
"#;
