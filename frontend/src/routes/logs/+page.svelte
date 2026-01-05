<script>
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    const severityOrder = { trace: -1, debug: 0, info: 1, warn: 2, error: 3 };
    let entries = $state([]);
    let seen = $state(new Set());
    let filterLevel = $state('all');
    let filterText = $state('');
    let sortMode = $state('time'); // 'time' or 'level'
    let updateInterval = $state('live'); // 'live', 1000, 5000, 30000
    let logBuffer = $state([]);
    let socket = null;
    let oldestTs = $state(null);
    let isLoadingMore = $state(false);
    let hasLoadedInitial = $state(false);
    let connectionError = $state(null);
    let isConnected = $state(false);
    let reconnectTimer = null;
    let flushTimer = null;

    function formatTime(ts) {
        if (!ts) return '';
        try {
            const d = new Date(ts);
            return d.getFullYear() + '-' +
                String(d.getMonth() + 1).padStart(2, '0') + '-' +
                String(d.getDate()).padStart(2, '0') + ' ' +
                String(d.getHours()).padStart(2, '0') + ':' +
                String(d.getMinutes()).padStart(2, '0') + ':' +
                String(d.getSeconds()).padStart(2, '0');
        } catch {
            return ts;
        }
    }

    function highlight(text, term) {
        if (!term || !text) return text;
        const safe = text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
        const pattern = term.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
        const regex = new RegExp(`(${pattern})`, 'gi');
        return safe.replace(regex, '<mark class="bg-[color-mix(in_srgb,var(--primary)_30%,transparent)] text-inherit rounded-sm px-0.5">$1</mark>');
    }

    function levelFor(event) {
        const t = (event.type || '').toLowerCase();
        if (t.includes('failed') || t.includes('error')) return 'error';
        if (t.includes('down') || t.includes('warn')) return 'warn';

        // For system_log events, extract level from the log line
        if (event.line) {
            // Match patterns like "11:21:04 DEBUG yanos::pkg:" or just "ERROR" at word boundary
            const levelMatch = event.line.match(/\b(ERROR|WARN|INFO|DEBUG|TRACE)\b/i);
            if (levelMatch) {
                const level = levelMatch[1].toLowerCase();
                if (level === 'error' || level === 'fail') return 'error';
                if (level === 'warn' || level === 'warning') return 'warn';
                if (level === 'debug') return 'debug';
                if (level === 'trace') return 'trace';
                return 'info';
            }
            // Fallback pattern matching
            if (/error|fail/i.test(event.line)) return 'error';
            if (/warn/i.test(event.line)) return 'warn';
        }
        return 'info';
    }

    function entryKey(ts, text) {
        return `${ts}|${text}`;
    }

    function asText(event) {
        switch (event.type) {
            case 'config_changed':
                return `ConfigChanged path=${event.path ?? ''}`;
            case 'system_log':
                return event.line ?? '';
            case 'service_started':
                return `ServiceStarted fmri=${event.fmri ?? ''}`;
            case 'service_stopped':
                return `ServiceStopped fmri=${event.fmri ?? ''}`;
            case 'service_failed':
                return `ServiceFailed fmri=${event.fmri ?? ''}`;
            case 'dataset_created':
                return `DatasetCreated name=${event.name ?? ''}`;
            case 'dataset_destroyed':
                return `DatasetDestroyed name=${event.name ?? ''}`;
            case 'link_up':
                return `LinkUp name=${event.name ?? ''}`;
            case 'link_down':
                return `LinkDown name=${event.name ?? ''}`;
            default:
                return JSON.stringify(event);
        }
    }

    function cleanMessage(msg) {
        // Strip Rust tracing timestamp and level if present
        // Matches: 2025-12-13T12:42:18.351226Z INFO ...
        msg = msg.replace(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(\.\d+)?Z\s+(INFO|WARN|ERROR|DEBUG|TRACE)\s+/, '');

        // Strip compact time and level format from tracing
        // Matches: 11:32:11 INFO ... or 11:32:11.123 DEBUG ...
        msg = msg.replace(/^\d{2}:\d{2}:\d{2}(\.\d+)?\s+(INFO|WARN|ERROR|DEBUG|TRACE)\s+/, '');

        // Strip Syslog header
        // Matches: Dec 9 09:36:37 localhost unix: ...
        msg = msg.replace(/^[A-Z][a-z]{2}\s+\d+\s+\d{2}:\d{2}:\d{2}\s+\S+\s+[^:]+:\s+/, '');

        // Strip Solaris/Illumos Message ID
        // Matches: [ID 123456 kern.info]
        msg = msg.replace(/\[ID \d+ [a-z0-9]+\.[a-z]+\]\s*/, '');
        
        return msg.replace(/\b(trace_id|span_id)=[a-f0-9]+\b/g, '').replace(/\s+/g, ' ').trim();
    }

    function processLog(logged) {
        const ev = logged.event ?? logged;
        const level = levelFor(ev);
        const rawText = asText(ev);
        const message = cleanMessage(rawText);
        const key = entryKey(logged.ts, rawText);

        if (seen.has(key)) return null;
        seen.add(key);
        
        return { ts: logged.ts, level, message, rawText };
    }

    function updateEntries(newItems) {
        if (newItems.length === 0) return;
        
        entries = [...newItems, ...entries].sort((a, b) => b.ts.localeCompare(a.ts)).slice(0, 1000);
        
        if (!oldestTs || newItems[newItems.length - 1].ts < oldestTs) {
            oldestTs = newItems[newItems.length - 1].ts;
        }

        if (seen.size > 1500) {
             seen = new Set(entries.map((entry) => entryKey(entry.ts, entry.rawText)));
        }
    }

    function flushLogs() {
        if (logBuffer.length === 0) return;

        const newEntries = [];
        for (const logged of logBuffer) {
            const entry = processLog(logged);
            if (entry) newEntries.push(entry);
        }
        logBuffer = [];
        updateEntries(newEntries);
    }

    function ingest(logged) {
        if (updateInterval === 'live') {
            const entry = processLog(logged);
            if (entry) updateEntries([entry]);
        } else {
            logBuffer.push(logged);
        }
    }

    $effect(() => {
        if (flushTimer) clearInterval(flushTimer);
        if (updateInterval !== 'live') {
            flushTimer = setInterval(flushLogs, parseInt(updateInterval));
        }
        return () => {
            if (flushTimer) clearInterval(flushTimer);
        };
    });

    async function loadInitial() {
        try {
            const res = await fetch('/api/v1/logs?limit=200', { credentials: 'include' });
            if (res.ok) {
                const data = await res.json();
                // Direct flush for initial load to avoid delay
                logBuffer.push(...data);
                flushLogs();
                connectionError = null;
                return true;
            } else if (res.status === 401) {
                connectionError = i18n.t('logs.unauth');
            } else {
                connectionError = `${i18n.t('logs.loadError')} (${res.status})`;
            }
        } catch (err) {
            connectionError = i18n.t('logs.loadError');
            console.error('Failed to load initial logs', err);
            return false;
        }
        return false;
    }

    async function loadMore() {
        if (isLoadingMore || !oldestTs) return;
        isLoadingMore = true;
        try {
            const res = await fetch(
                `/api/v1/logs?before=${encodeURIComponent(oldestTs)}&limit=200`,
                { credentials: 'include' },
            );
            if (res.ok) {
                const data = await res.json();
                logBuffer.push(...data);
                flushLogs();
            } else if (res.status === 401) {
                connectionError = i18n.t('logs.unauth');
            }
        } catch (err) {
            console.error('Failed to load more logs', err);
            connectionError = i18n.t('logs.loadError');
        } finally {
            isLoadingMore = false;
        }
    }
    
    // ... (rest of functions: scheduleReconnect, connect, $effect for connect, viewEntries)



    function scheduleReconnect() {
        if (reconnectTimer || !auth.isAuthenticated) return;
        reconnectTimer = setTimeout(() => {
            reconnectTimer = null;
            connect();
        }, 1500);
    }

    function connect() {
        if (!auth.isAuthenticated) {
            if (socket) {
                socket.close();
                socket = null;
            }
            isConnected = false;
            return;
        }

        if (socket && socket.readyState <= 1) return;

        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        socket = new WebSocket(`${protocol}://${location.host}/api/v1/events`);

        socket.onopen = () => {
            isConnected = true;
            connectionError = null;
            if (!hasLoadedInitial) {
                loadInitial().then((ok) => {
                    if (ok) {
                        hasLoadedInitial = true;
                    }
                });
            }
        };

        socket.onmessage = (evt) => {
            try {
                const event = JSON.parse(evt.data);
                ingest(event);
            } catch (err) {
                console.error('Failed to parse event', err);
            }
        };

        socket.onerror = () => {
            connectionError = i18n.t('logs.streamError');
            isConnected = false;
            scheduleReconnect();
        };

        socket.onclose = () => {
            socket = null;
            isConnected = false;
            scheduleReconnect();
        };
    }

    $effect(() => {
        let cancelled = false;

        if (auth.isAuthenticated && !hasLoadedInitial) {
            loadInitial().then((ok) => {
                if (!cancelled && ok) {
                    hasLoadedInitial = true;
                }
            });
        }

        if (!auth.isAuthenticated) {
            entries = [];
            seen = new Set();
            oldestTs = null;
            hasLoadedInitial = false;
            connectionError = null;
        }

        connect();
        return () => {
            if (socket) {
                socket.close();
                socket = null;
            }
            if (reconnectTimer) {
                clearTimeout(reconnectTimer);
                reconnectTimer = null;
            }
            cancelled = true;
        };
    });

    let viewEntries = $derived.by(() => {
        let filtered = entries;

        if (filterLevel !== 'all') {
            filtered = filtered.filter((e) => e.level === filterLevel);
        }

        if (filterText.trim()) {
            const term = filterText.toLowerCase();
            filtered = filtered.filter((e) => e.message.toLowerCase().includes(term));
        }

        if (sortMode === 'level') {
            return [...filtered].sort((a, b) => {
                const diff = severityOrder[b.level] - severityOrder[a.level];
                if (diff !== 0) return diff;
                return b.ts.localeCompare(a.ts);
            });
        }

        return filtered;
    });
</script>

<section class="flex flex-col h-full gap-4 overflow-hidden">
    <header class="flex items-center justify-between shrink-0">
        <div>
            <h1 class="text-2xl font-bold text-text-main">{i18n.t('logs.title')}</h1>
            <p class="text-text-muted">{i18n.t('logs.subtitle')}</p>
        </div>
        <div class="flex items-center gap-2">
            <input
                type="text"
                placeholder="Filter logs..."
                class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-1 w-48 focus:outline-none focus:border-primary focus:ring-1 focus:ring-primary transition-colors"
                bind:value={filterText}
            />
            <select
                class="border border-border-main bg-bg-card text-text-main text-sm rounded px-2 py-1"
                bind:value={updateInterval}
            >
                <option value="live">Live</option>
                <option value="1000">1s</option>
                <option value="5000">5s</option>
                <option value="30000">30s</option>
            </select>
            <select
                class="border border-border-main bg-bg-card text-text-main text-sm rounded px-2 py-1"
                bind:value={filterLevel}
            >
                <option value="all">{i18n.t('logs.filterAll')}</option>
                <option value="error">{i18n.t('logs.filterError')}</option>
                <option value="warn">{i18n.t('logs.filterWarn')}</option>
                <option value="info">{i18n.t('logs.filterInfo')}</option>
                <option value="debug">Debug</option>
                <option value="trace">Trace</option>
            </select>
            <select
                class="border border-border-main bg-bg-card text-text-main text-sm rounded px-2 py-1"
                bind:value={sortMode}
            >
                <option value="time">{i18n.t('logs.sortTime')}</option>
                <option value="level">{i18n.t('logs.sortLevel')}</option>
            </select>
        </div>
    </header>

    {#if !auth.isAuthenticated}
        <div class="bg-amber-50 text-amber-800 border border-amber-200 rounded p-4 shrink-0">
            {i18n.t('logs.unauth')}
        </div>
    {:else}
        <div class="border border-border-main rounded bg-bg-card flex-1 overflow-hidden flex flex-col">
            {#if viewEntries.length === 0}
                <div class="p-4 text-text-muted">{i18n.t('logs.none')}</div>
            {:else}
                <div 
                    class="overflow-auto flex-1 w-full"
                    onscroll={(e) => {
                        if (e.target.scrollTop === 0) {
                            loadMore();
                        }
                    }}
                >
                    <table class="w-full text-left text-sm font-mono border-collapse">
                        <thead class="bg-bg-sidebar sticky top-0 shadow-sm">
                            <tr>
                                <th class="p-2 w-48 font-semibold text-text-muted border-b border-border-main">Timestamp</th>
                                <th class="p-2 w-24 font-semibold text-text-muted border-b border-border-main">Level</th>
                                <th class="p-2 font-semibold text-text-muted border-b border-border-main">Message</th>
                            </tr>
                        </thead>
                        <tbody>
                            {#each viewEntries as entry}
                                <tr class="hover:bg-black/5 border-b border-border-main last:border-0">
                                    <td class="p-2 text-text-muted whitespace-nowrap align-top">{formatTime(entry.ts)}</td>
                                    <td class="p-2 align-top">
                                        <span class={`px-1.5 py-0.5 rounded text-xs font-bold uppercase ${
                                            entry.level === 'error' ? 'bg-red-100 text-red-700' :
                                            entry.level === 'warn' ? 'bg-amber-100 text-amber-700' :
                                            entry.level === 'debug' ? 'bg-gray-100 text-gray-600' :
                                            entry.level === 'trace' ? 'bg-gray-50 text-gray-400' :
                                            'bg-blue-50 text-blue-700'
                                        }`}>
                                            {entry.level}
                                        </span>
                                    </td>
                                    <td class="p-2 text-text-main break-words align-top">{@html highlight(entry.message, filterText)}</td>
                                </tr>
                            {/each}
                        </tbody>
                    </table>
                </div>
            {/if}
        </div>
    {/if}
</section>
