<script>
    import { beforeNavigate, goto } from '$app/navigation';
    import { page } from '$app/stores';
    import { auth } from '$lib/auth.svelte.js';
    import PendingChangesModal from '$lib/components/PendingChangesModal.svelte';
    import { availableLangs, i18n } from '$lib/i18n.svelte.js';
    import { availableThemes, theme } from '$lib/theme.svelte.js';

    let appliedLang = $state(i18n.current);
    let appliedTheme = $state(theme.current);

    // Telemetry receiver endpoints - each independent
    let appliedTempoEndpoint = $state('');
    let appliedLokiEndpoint = $state('');
    let appliedPrometheusEndpoint = $state('');

    let pendingLang = $state(i18n.current);
    let pendingTheme = $state(theme.current);
    let pendingTempoEndpoint = $state('');
    let pendingLokiEndpoint = $state('');
    let pendingPrometheusEndpoint = $state('');

    let showPendingModal = $state(false);
    let pendingUrl = $state(null);
    let savingTelemetry = $state(false);
    let saveStatus = $state(null); // { type: 'success' | 'error', message: string }
    let telemetryLoaded = $state(false);

    // Test status per endpoint
    let testingEndpoint = $state(null); // 'traces' | 'logs' | 'metrics' | null
    let testResults = $state({}); // { traces?: {ok, error}, logs?: {...}, metrics?: {...} }

    const telemetryDirty = $derived(
        pendingTempoEndpoint !== appliedTempoEndpoint ||
        pendingLokiEndpoint !== appliedLokiEndpoint ||
        pendingPrometheusEndpoint !== appliedPrometheusEndpoint
    );
    const isDirty = $derived(
        pendingLang !== appliedLang ||
        pendingTheme !== appliedTheme ||
        telemetryDirty
    );

    async function loadTelemetrySettings() {
        if (!auth.isAuthenticated || telemetryLoaded) return;
        try {
            const res = await fetch('/api/v1/settings/telemetry', { credentials: 'include' });
            if (res.ok) {
                const data = await res.json();
                appliedTempoEndpoint = data.tempo_endpoint ?? '';
                appliedLokiEndpoint = data.loki_endpoint ?? '';
                appliedPrometheusEndpoint = data.prometheus_endpoint ?? '';
                pendingTempoEndpoint = appliedTempoEndpoint;
                pendingLokiEndpoint = appliedLokiEndpoint;
                pendingPrometheusEndpoint = appliedPrometheusEndpoint;
            }
        } catch (e) {
            console.error('Failed to load telemetry settings', e);
        } finally {
            telemetryLoaded = true;
        }
    }

    async function testEndpoint(type, endpoint) {
        if (!endpoint.trim()) return;
        testingEndpoint = type;
        testResults = { ...testResults, [type]: null };

        try {
            const token = auth.readCsrfFromCookie?.() ?? auth.csrfToken;
            const res = await fetch('/api/v1/settings/telemetry/test', {
                method: 'POST',
                credentials: 'include',
                headers: {
                    'Content-Type': 'application/json',
                    ...(token ? { 'X-CSRF-TOKEN': token } : {})
                },
                body: JSON.stringify({ endpoint: endpoint.trim() })
            });

            if (res.ok) {
                const data = await res.json();
                testResults = { ...testResults, [type]: { ok: data.reachable, error: data.error } };
            } else {
                testResults = { ...testResults, [type]: { ok: false, error: 'Request failed' } };
            }
        } catch (e) {
            testResults = { ...testResults, [type]: { ok: false, error: e.message } };
        } finally {
            testingEndpoint = null;
        }
    }

    async function saveTelemetry() {
        if (!telemetryDirty) return;
        savingTelemetry = true;
        saveStatus = null;
        const token = auth.readCsrfFromCookie?.() ?? auth.csrfToken;
        try {
            const res = await fetch('/api/v1/settings/telemetry', {
                method: 'PUT',
                credentials: 'include',
                headers: {
                    'Content-Type': 'application/json',
                    ...(token ? { 'X-CSRF-TOKEN': token } : {})
                },
                body: JSON.stringify({
                    tempo_endpoint: pendingTempoEndpoint.trim() || null,
                    loki_endpoint: pendingLokiEndpoint.trim() || null,
                    prometheus_endpoint: pendingPrometheusEndpoint.trim() || null
                })
            });

            if (res.ok) {
                const data = await res.json();
                appliedTempoEndpoint = data.tempo_endpoint ?? '';
                appliedLokiEndpoint = data.loki_endpoint ?? '';
                appliedPrometheusEndpoint = data.prometheus_endpoint ?? '';
                pendingTempoEndpoint = appliedTempoEndpoint;
                pendingLokiEndpoint = appliedLokiEndpoint;
                pendingPrometheusEndpoint = appliedPrometheusEndpoint;
                saveStatus = { type: 'success', message: i18n.t('settings.telemetrySaved') };
            } else {
                const err = await res.json();
                saveStatus = { type: 'error', message: err.error || i18n.t('settings.telemetryTestFail') };
            }
        } catch (e) {
            console.error('Failed to save telemetry settings', e);
            saveStatus = { type: 'error', message: i18n.t('settings.telemetryTestFail') };
        } finally {
            savingTelemetry = false;
        }
    }

    async function applyAll() {
        i18n.setLang(pendingLang);
        appliedLang = pendingLang;

        theme.setTheme(pendingTheme);
        appliedTheme = pendingTheme;

        if (telemetryDirty) {
            await saveTelemetry();
        }

        pendingLang = appliedLang;
        pendingTheme = appliedTheme;
    }

    async function resolvePending(save) {
        const target = pendingUrl;
        pendingUrl = null;
        showPendingModal = false;

        if (save) {
            await applyAll();
        }

        if (save && target) {
            goto(target);
        }
    }

    beforeNavigate((nav) => {
        if (!isDirty || showPendingModal) {
            return;
        }
        nav.cancel();
        const dest = nav.destination?.url;
        pendingUrl = dest ? `${dest.pathname}${dest.search}${dest.hash}` : null;
        showPendingModal = true;
    });

    $effect(() => {
        if (auth.isInitialized && !auth.isAuthenticated && $page.url.pathname !== '/login') {
            goto('/login');
        }
    });

    $effect(() => {
        if (auth.isInitialized && auth.isAuthenticated && $page.url.pathname === '/login') {
            goto('/');
        }
    });

    $effect(() => {
        if (auth.isInitialized && auth.isAuthenticated) {
            loadTelemetrySettings();
        }
    });
</script>

<section class="p-6 space-y-8">
    <header>
        <h1 class="text-2xl font-bold text-text-main">{i18n.t('settings.title')}</h1>
        <p class="text-text-muted">{i18n.t('settings.subtitle')}</p>
    </header>

    <section id="ui" class="space-y-4 border border-border-main rounded bg-bg-card p-4">
        <div class="flex items-center justify-between border-b border-border-main pb-3">
            <div>
                <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.uiSection')}</h2>
                <p class="text-text-muted text-sm">{i18n.t('settings.subtitle')}</p>
            </div>
        </div>

        <div class="flex flex-col gap-4">
            <div class="flex flex-col sm:flex-row sm:items-center gap-2">
                <div class="text-text-main font-semibold sm:w-1/3" title={i18n.t('settings.languageLabel')}>
                    {i18n.t('settings.languageLabel')}
                </div>
                <div class="sm:w-2/3 flex sm:justify-start">
                    <select
                        class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 w-full sm:w-64"
                        bind:value={pendingLang}
                        title={i18n.t('settings.languageLabel')}
                    >
                        {#each availableLangs as l}
                            <option value={l.id}>{l.name}</option>
                        {/each}
                    </select>
                </div>
            </div>

            <div class="flex flex-col sm:flex-row sm:items-center gap-2 border-t border-border-main pt-3 mt-1">
                <div class="text-text-main font-semibold sm:w-1/3" title={i18n.t('settings.themeLabel')}>
                    {i18n.t('settings.themeLabel')}
                </div>
                <div class="sm:w-2/3 flex sm:justify-start">
                    <select
                        class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 w-full sm:w-64"
                        bind:value={pendingTheme}
                        title={i18n.t('settings.themeLabel')}
                    >
                        {#each availableThemes as t}
                            <option value={t.id}>{t.name}</option>
                        {/each}
                    </select>
                </div>
            </div>
        </div>
        <div class="flex justify-end">
            <button
                type="button"
                class="px-3 py-2 bg-primary text-primary-fg rounded text-sm hover:bg-primary-hover"
                onclick={applyAll}
            >
                {i18n.t('settings.apply')}
            </button>
        </div>
    </section>

    <section id="notifications" class="space-y-3 border border-border-main rounded bg-bg-card p-4">
        <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.notifications')}</h2>
        <p class="text-text-muted text-sm">{i18n.t('settings.placeholder')}</p>
        <div class="h-32 bg-bg-main border border-dashed border-border-main rounded"></div>
    </section>

    <section id="integrations" class="space-y-4 border border-border-main rounded bg-bg-card p-4">
        <div class="flex items-start justify-between">
            <div>
                <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.telemetry')}</h2>
                <p class="text-text-muted text-sm">{i18n.t('settings.telemetrySubtitle')}</p>
            </div>
        </div>

        <div class="space-y-4">
            <!-- Tempo Endpoint (Traces) -->
            <div class="space-y-2">
                <label class="text-sm text-text-main font-semibold" for="tempo-endpoint">
                    {i18n.t('settings.tempoEndpoint')}
                </label>
                <div class="flex flex-col sm:flex-row gap-2">
                    <input
                        id="tempo-endpoint"
                        class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 flex-1"
                        placeholder={i18n.t('settings.tempoPlaceholder')}
                        bind:value={pendingTempoEndpoint}
                    />
                    <button
                        class="px-3 py-2 border border-border-main rounded text-sm hover:bg-bg-main disabled:opacity-50"
                        onclick={() => testEndpoint('tempo', pendingTempoEndpoint)}
                        disabled={testingEndpoint === 'tempo' || !pendingTempoEndpoint.trim()}
                        type="button"
                    >
                        {testingEndpoint === 'tempo' ? '...' : i18n.t('settings.test')}
                    </button>
                </div>
                {#if testResults.tempo}
                    <p class={`text-xs ${testResults.tempo.ok ? 'text-green-600' : 'text-red-600'}`}>
                        {testResults.tempo.ok ? i18n.t('settings.reachable') : testResults.tempo.error}
                    </p>
                {/if}
            </div>

            <!-- Loki Endpoint (Logs) -->
            <div class="space-y-2 border-t border-border-main pt-4">
                <label class="text-sm text-text-main font-semibold" for="loki-endpoint">
                    {i18n.t('settings.lokiEndpoint')}
                </label>
                <div class="flex flex-col sm:flex-row gap-2">
                    <input
                        id="loki-endpoint"
                        class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 flex-1"
                        placeholder={i18n.t('settings.lokiPlaceholder')}
                        bind:value={pendingLokiEndpoint}
                    />
                    <button
                        class="px-3 py-2 border border-border-main rounded text-sm hover:bg-bg-main disabled:opacity-50"
                        onclick={() => testEndpoint('loki', pendingLokiEndpoint)}
                        disabled={testingEndpoint === 'loki' || !pendingLokiEndpoint.trim()}
                        type="button"
                    >
                        {testingEndpoint === 'loki' ? '...' : i18n.t('settings.test')}
                    </button>
                </div>
                {#if testResults.loki}
                    <p class={`text-xs ${testResults.loki.ok ? 'text-green-600' : 'text-red-600'}`}>
                        {testResults.loki.ok ? i18n.t('settings.reachable') : testResults.loki.error}
                    </p>
                {/if}
            </div>

            <!-- Prometheus Endpoint (Metrics) -->
            <div class="space-y-2 border-t border-border-main pt-4">
                <label class="text-sm text-text-main font-semibold" for="prometheus-endpoint">
                    {i18n.t('settings.prometheusEndpoint')}
                </label>
                <div class="flex flex-col sm:flex-row gap-2">
                    <input
                        id="prometheus-endpoint"
                        class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 flex-1"
                        placeholder={i18n.t('settings.prometheusPlaceholder')}
                        bind:value={pendingPrometheusEndpoint}
                    />
                    <button
                        class="px-3 py-2 border border-border-main rounded text-sm hover:bg-bg-main disabled:opacity-50"
                        onclick={() => testEndpoint('prometheus', pendingPrometheusEndpoint)}
                        disabled={testingEndpoint === 'prometheus' || !pendingPrometheusEndpoint.trim()}
                        type="button"
                    >
                        {testingEndpoint === 'prometheus' ? '...' : i18n.t('settings.test')}
                    </button>
                </div>
                {#if testResults.prometheus}
                    <p class={`text-xs ${testResults.prometheus.ok ? 'text-green-600' : 'text-red-600'}`}>
                        {testResults.prometheus.ok ? i18n.t('settings.reachable') : testResults.prometheus.error}
                    </p>
                {/if}
            </div>

            <p class="text-xs text-text-muted">{i18n.t('settings.telemetryDisabled')}</p>

            <div class="flex items-center justify-end gap-3 pt-2 border-t border-border-main">
                {#if saveStatus}
                    <span class={`text-xs ${saveStatus.type === 'success' ? 'text-green-600' : 'text-red-600'}`}>
                        {saveStatus.message}
                    </span>
                {/if}
                <button
                    class="px-4 py-2 bg-primary text-primary-fg rounded text-sm hover:bg-primary-hover disabled:opacity-50"
                    onclick={saveTelemetry}
                    disabled={savingTelemetry || !telemetryDirty}
                    type="button"
                >
                    {savingTelemetry ? '...' : i18n.t('settings.apply')}
                </button>
            </div>
        </div>
    </section>

    <section id="about" class="space-y-3 border border-border-main rounded bg-bg-card p-4">
        <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.about')}</h2>
        <p class="text-text-muted text-sm">{i18n.t('settings.placeholder')}</p>
        <div class="h-32 bg-bg-main border border-dashed border-border-main rounded"></div>
    </section>

    {#if showPendingModal}
        <PendingChangesModal
            title={i18n.t('settings.title')}
            message={i18n.t('settings.confirmLeave')}
            cancelLabel={i18n.t('nav.cancel')}
            confirmLabel={i18n.t('settings.apply')}
            onCancel={() => resolvePending(false)}
            onConfirm={() => resolvePending(true)}
        />
    {/if}
</section>
