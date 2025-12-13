<script>
    import { beforeNavigate, goto } from '$app/navigation';
    import { page } from '$app/stores';
    import { auth } from '$lib/auth.svelte.js';
    import PendingChangesModal from '$lib/components/PendingChangesModal.svelte';
    import { availableLangs, i18n } from '$lib/i18n.svelte.js';
    import { availableThemes, theme } from '$lib/theme.svelte.js';

    let appliedLang = $state(i18n.current);
    let appliedTheme = $state(theme.current);
    let appliedOtlpEndpoint = $state('');

    let pendingLang = $state(i18n.current);
    let pendingTheme = $state(theme.current);
    let pendingOtlpEndpoint = $state('');

    let showPendingModal = $state(false);
    let pendingUrl = $state(null);
    let savingTelemetry = $state(false);
    let testingTelemetry = $state(false);
    let testStatus = $state(null);
    let telemetryLoaded = $state(false);

    const telemetryDirty = $derived(pendingOtlpEndpoint !== appliedOtlpEndpoint);
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
                appliedOtlpEndpoint = data.otlp_endpoint ?? '';
                pendingOtlpEndpoint = appliedOtlpEndpoint;
            }
        } catch (e) {
            console.error('Failed to load telemetry settings', e);
        } finally {
            telemetryLoaded = true;
        }
    }

    async function saveTelemetry() {
        if (!telemetryDirty) return;
        savingTelemetry = true;
        testStatus = null;
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
                    otlp_endpoint: pendingOtlpEndpoint.trim() === '' ? null : pendingOtlpEndpoint.trim()
                })
            });

            if (res.ok) {
                const data = await res.json();
                appliedOtlpEndpoint = data.otlp_endpoint ?? '';
                pendingOtlpEndpoint = appliedOtlpEndpoint;
            }
        } catch (e) {
            console.error('Failed to save telemetry settings', e);
        } finally {
            savingTelemetry = false;
        }
    }

    async function testTelemetry() {
        if (!browser || !pendingOtlpEndpoint.trim()) {
            testStatus = 'fail';
            return;
        }
        testingTelemetry = true;
        testStatus = null;
        const token = auth.readCsrfFromCookie?.() ?? auth.csrfToken;
        try {
            const res = await fetch('/api/v1/settings/telemetry/test', {
                method: 'POST',
                credentials: 'include',
                headers: {
                    'Content-Type': 'application/json',
                    ...(token ? { 'X-CSRF-TOKEN': token } : {})
                },
                body: JSON.stringify({ otlp_endpoint: pendingOtlpEndpoint.trim() })
            });
            testStatus = res.ok ? 'ok' : 'fail';
        } catch (e) {
            console.error('Failed to test telemetry endpoint', e);
            testStatus = 'fail';
        } finally {
            testingTelemetry = false;
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

        <div class="space-y-2">
            <label class="text-sm text-text-main font-semibold" for="otlp-endpoint">
                {i18n.t('settings.otlpEndpoint')}
            </label>
            <div class="flex flex-col sm:flex-row gap-3">
                <input
                    id="otlp-endpoint"
                    class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 flex-1"
                    placeholder={i18n.t('settings.otlpPlaceholder')}
                    bind:value={pendingOtlpEndpoint}
                />
                <button
                    class="px-4 py-2 bg-bg-main text-text-main border border-border-main rounded text-sm hover:border-primary disabled:opacity-50"
                    type="button"
                    onclick={testTelemetry}
                    disabled={testingTelemetry}
                >
                    {testingTelemetry ? '...' : i18n.t('settings.telemetryTest')}
                </button>
            </div>
            <p class="text-xs text-text-muted">{i18n.t('settings.telemetryDisabled')}</p>
            {#if testStatus === 'ok'}
                <p class="text-xs text-green-600">{i18n.t('settings.telemetryTestSuccess')}</p>
            {:else if testStatus === 'fail'}
                <p class="text-xs text-red-600">{i18n.t('settings.telemetryTestFail')}</p>
            {/if}
            <div class="flex justify-end pt-2 mt-2">
                <button
                    class="px-4 py-2 bg-primary text-primary-fg rounded text-sm hover:bg-primary-hover disabled:opacity-50"
                    onclick={saveTelemetry}
                    disabled={savingTelemetry}
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
