<script>
    import { beforeNavigate, goto } from '$app/navigation';
    import PendingChangesModal from '$lib/components/PendingChangesModal.svelte';
    import { availableLangs, i18n } from '$lib/i18n.svelte.js';
    import { availableThemes, theme } from '$lib/theme.svelte.js';

    let appliedLang = $state(i18n.current);
    let appliedTheme = $state(theme.current);
    let pendingLang = $state(i18n.current);
    let pendingTheme = $state(theme.current);
    let showPendingModal = $state(false);
    let pendingUrl = $state(null);

    const isDirty = $derived(pendingLang !== appliedLang || pendingTheme !== appliedTheme);

    function applyAll() {
        i18n.setLang(pendingLang);
        appliedLang = pendingLang;

        theme.setTheme(pendingTheme);
        appliedTheme = pendingTheme;

        pendingLang = appliedLang;
        pendingTheme = appliedTheme;
    }

    function resolvePending(save) {
        const target = pendingUrl;
        pendingUrl = null;
        showPendingModal = false;

        if (save) {
            applyAll();
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

            <div class="flex flex-col sm:flex-row sm:items-center gap-2">
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

    <section id="integrations" class="space-y-3 border border-border-main rounded bg-bg-card p-4">
        <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.integrations')}</h2>
        <p class="text-text-muted text-sm">{i18n.t('settings.placeholder')}</p>
        <div class="h-32 bg-bg-main border border-dashed border-border-main rounded"></div>
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
