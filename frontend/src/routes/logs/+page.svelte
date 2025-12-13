<script>
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    const severityOrder = { info: 0, warn: 1, error: 2 };
    let entries = $state([]);
    let seen = $state(new Set());
    let filterLevel = $state('all');
    let sortMode = $state('time'); // 'time' or 'level'
    let socket = null;
    let oldestTs = $state(null);
    let isLoadingMore = $state(false);
    let hasLoadedInitial = $state(false);
    let connectionError = $state(null);
    let isConnected = $state(false);
    let reconnectTimer = null;

    function levelFor(event) {
        const t = (event.type || '').toLowerCase();
        if (t.includes('failed') || t.includes('error')) return 'error';
        if (t.includes('down') || t.includes('warn')) return 'warn';
        if (event.line && /error|fail/i.test(event.line)) return 'error';
        if (event.line && /warn/i.test(event.line)) return 'warn';
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

    function ingest(logged) {
        const ev = logged.event ?? logged;
        const level = levelFor(ev);
        const text = `[${logged.ts}] [${level.toUpperCase()}] ${asText(ev)}`;
        const key = entryKey(logged.ts, text);

        if (seen.has(key)) return;
        seen.add(key);

        entries = [{ ts: logged.ts, level, text }, ...entries].slice(0, 1000);
        if (!oldestTs || logged.ts < oldestTs) {
            oldestTs = logged.ts;
        }

        if (seen.size > 1500) {
            seen = new Set(entries.map((entry) => entryKey(entry.ts, entry.text)));
        }
    }

    async function loadInitial() {
        try {
            const res = await fetch('/api/v1/logs?limit=200', { credentials: 'include' });
            if (res.ok) {
                const data = await res.json();
                data.forEach((log) => ingest(log));
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
                data.forEach((log) => ingest(log));
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

    let viewEntries = $derived(() => {
        let filtered =
            filterLevel === 'all'
                ? entries
                : entries.filter((e) => e.level === filterLevel);

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

<section class="p-6 space-y-4">
    <header class="flex items-center justify-between">
        <div>
            <h1 class="text-2xl font-bold text-text-main">{i18n.t('logs.title')}</h1>
            <p class="text-text-muted">{i18n.t('logs.subtitle')}</p>
        </div>
        <div class="flex items-center gap-2">
            <div
                class={`text-xs px-2 py-1 rounded border ${
                    isConnected
                        ? 'border-emerald-300 bg-emerald-50 text-emerald-700'
                        : 'border-amber-300 bg-amber-50 text-amber-700'
                }`}
            >
                {isConnected ? i18n.t('logs.liveConnected') : i18n.t('logs.liveDisconnected')}
            </div>
            <select
                class="border border-border-main bg-bg-card text-text-main text-sm rounded px-2 py-1"
                bind:value={filterLevel}
            >
                <option value="all">{i18n.t('logs.filterAll')}</option>
                <option value="error">{i18n.t('logs.filterError')}</option>
                <option value="warn">{i18n.t('logs.filterWarn')}</option>
                <option value="info">{i18n.t('logs.filterInfo')}</option>
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

    {#if connectionError && auth.isAuthenticated}
        <div class="bg-amber-50 text-amber-800 border border-amber-200 rounded p-3 text-sm">
            {connectionError}
        </div>
    {/if}

    {#if !auth.isAuthenticated}
        <div class="bg-amber-50 text-amber-800 border border-amber-200 rounded p-4">
            {i18n.t('logs.unauth')}
        </div>
    {:else}
        <div class="border border-border-main rounded bg-bg-card overflow-hidden">
            {#if viewEntries.length === 0}
                <div class="p-4 text-text-muted">{i18n.t('logs.none')}</div>
            {:else}
                <pre
                    class="m-0 p-4 text-sm font-mono text-text-main whitespace-pre-wrap leading-relaxed max-h-[60vh] overflow-auto"
                    onscroll={(e) => {
                        if (e.target.scrollTop === 0) {
                            loadMore();
                        }
                    }}
                >
{#each viewEntries as entry}
{entry.text}
{/each}</pre>
            {/if}
        </div>
    {/if}
</section>
