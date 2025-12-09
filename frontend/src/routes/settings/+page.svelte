<script>
    import ThemeSelector from '$lib/components/ThemeSelector.svelte';
    import { availableThemes, theme } from '$lib/theme.svelte.js';
    import { availableLangs, i18n } from '$lib/i18n.svelte.js';

    let current = $derived(theme.current);
    let lang = $state(i18n.current);

    function choose(id) {
        theme.setTheme(id);
    }

    function chooseLang(id) {
        i18n.setLang(id);
        lang = id;
    }
</script>

<section class="p-6 space-y-6">
    <header>
        <h1 class="text-2xl font-bold text-text-main">{i18n.t('settings.title')}</h1>
        <p class="text-text-muted">{i18n.t('settings.subtitle')}</p>
    </header>

    <div class="border border-border-main rounded bg-bg-card">
        <div class="flex items-center justify-between px-4 py-3 border-b border-border-main">
            <div>
                <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.theme')}</h2>
                <p class="text-text-muted text-sm">{i18n.t('settings.themeSubtitle')}</p>
            </div>
            <ThemeSelector />
        </div>
        <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-3 p-4">
            {#each availableThemes as t}
                <button
                    type="button"
                    onclick={() => choose(t.id)}
                    class="border rounded p-3 text-left transition-colors
                    {current === t.id ? 'border-primary bg-primary/10 text-primary' : 'border-border-main hover:border-primary'}"
                >
                    <div class="font-semibold">{t.name}</div>
                    <div class="text-xs text-text-muted mt-1">
                        {current === t.id ? i18n.t('settings.themeActive') : i18n.t('settings.themeActivate')}
                    </div>
                </button>
            {/each}
        </div>
    </div>

    <div class="border border-border-main rounded bg-bg-card">
        <div class="flex items-center justify-between px-4 py-3 border-b border-border-main">
            <div>
                <h2 class="text-lg font-semibold text-text-main">{i18n.t('settings.language')}</h2>
                <p class="text-text-muted text-sm">{i18n.t('settings.subtitle')}</p>
            </div>
        </div>
        <div class="p-4 flex flex-wrap gap-2">
            {#each availableLangs as l}
                <button
                    type="button"
                    onclick={() => chooseLang(l.id)}
                    class="px-3 py-2 border rounded text-sm
                    {lang === l.id ? 'border-primary bg-primary/10 text-primary' : 'border-border-main hover:border-primary'}"
                >
                    {l.name}
                </button>
            {/each}
        </div>
    </div>
</section>
