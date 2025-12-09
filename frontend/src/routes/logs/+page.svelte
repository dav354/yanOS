<script>
    import { auth } from '$lib/auth.svelte.js';
    import { i18n } from '$lib/i18n.svelte.js';

    const severityOrder = { info: 0, warn: 1, error: 2 };
    let entries = $state([]);
    let filterLevel = $state('all');
    let sortMode = $state('time'); // 'time' or 'level'
    let socket = null;

    function levelFor(event) {
        const t = (event.type || '').toLowerCase();
        if (t.includes('failed') || t.includes('error')) return 'error';
        if (t.includes('down')) return 'warn';
        return 'info';
    }

    function asText(event) {
        switch (event.type) {
            case 'config_changed':
                return `ConfigChanged path=${event.path ?? ''}`;
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

    function connect() {
        if (!auth.isAuthenticated) {
            if (socket) {
                socket.close();
                socket = null;
            }
            return;
        }

        if (socket && socket.readyState <= 1) return;

        const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
        socket = new WebSocket(`${protocol}://${location.host}/api/v1/events`);

        socket.onmessage = (evt) => {
            try {
                const event = JSON.parse(evt.data);
                const level = levelFor(event);
                const ts = new Date().toISOString();
                const text = `[${ts}] [${level.toUpperCase()}] ${asText(event)}`;
                entries = [{ ts, level, text }, ...entries].slice(0, 200);
            } catch (err) {
                console.error('Failed to parse event', err);
            }
        };

        socket.onclose = () => {
            socket = null;
        };
    }

    $effect(() => {
        connect();
        return () => {
            if (socket) {
                socket.close();
                socket = null;
            }
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

    {#if !auth.isAuthenticated}
        <div class="bg-amber-50 text-amber-800 border border-amber-200 rounded p-4">
            {i18n.t('logs.unauth')}
        </div>
    {:else}
        <div class="border border-border-main rounded bg-bg-card overflow-hidden">
            {#if viewEntries.length === 0}
                <div class="p-4 text-text-muted">{i18n.t('logs.none')}</div>
            {:else}
                <pre class="m-0 p-4 text-sm font-mono text-text-main whitespace-pre-wrap leading-relaxed">
{#each viewEntries as entry}
{entry.text}
{/each}</pre>
            {/if}
        </div>
    {/if}
</section>
