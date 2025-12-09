<script>
    import ThemeSelector from '$lib/components/ThemeSelector.svelte';
    import { availableThemes, theme } from '$lib/theme.svelte.js';
    import { availableLangs, i18n } from '$lib/i18n.svelte.js';

    const sections = [
        { id: 'ui', title: i18n.t('settings.uiSection') },
        { id: 'notifications', title: i18n.t('settings.notifications') },
        { id: 'integrations', title: i18n.t('settings.integrations') },
        { id: 'about', title: i18n.t('settings.about') },
    ];

    let current = $state(theme.current);
    let lang = $state(i18n.current);
    let activeSection = $state('ui');
    let pendingLang = $state(lang);
    let pendingTheme = $state(current);

    function choose(id) {
        pendingTheme = id;
    }

    function chooseLang(id) {
        pendingLang = id;
    }

    function applyTheme() {
        theme.setTheme(pendingTheme);
        current = pendingTheme;
    }

    function applyLang() {
        i18n.setLang(pendingLang);
        lang = pendingLang;
    }

    function scrollTo(id) {
        const el = document.getElementById(id);
        if (el) {
            el.scrollIntoView({ behavior: 'smooth', block: 'start' });
            activeSection = id;
        }
    }
</script>

<section class="p-6 grid grid-cols-1 lg:grid-cols-4 gap-6">
    <div class="lg:col-span-1">
        <div class="sticky top-4 space-y-2 border border-border-main rounded bg-bg-card p-3">
            <h2 class="text-sm font-semibold text-text-muted uppercase tracking-wide">{i18n.t('settings.title')}</h2>
            <ul class="space-y-1">
                {#each sections as section}
                    <li>
                        <button
                            class={"w-full text-left px-3 py-2 rounded text-sm transition-colors "
                                + (activeSection === section.id ? 'bg-primary text-primary-fg' : 'hover:bg-bg-main text-text-main')}
                            onclick={() => scrollTo(section.id)}
                            type="button"
                        >
                            {section.title}
                        </button>
                    </li>
                {/each}
            </ul>
        </div>
    </div>

    <div class="lg:col-span-3 space-y-8 max-h-[80vh] overflow-y-auto pr-1">
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

            <div class="grid grid-cols-1 gap-4">
                <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-3 border border-border-main rounded p-3">
                    <div class="text-text-main font-semibold">{i18n.t('settings.languageLabel')}</div>
                    <div class="flex items-center gap-2">
                        <select
                            class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 min-w-[160px]"
                            bind:value={pendingLang}
                        >
                            {#each availableLangs as l}
                                <option value={l.id}>{l.name}</option>
                            {/each}
                        </select>
                        <button
                            type="button"
                            class="px-3 py-2 bg-primary text-primary-fg rounded text-sm hover:bg-primary-hover"
                            onclick={applyLang}
                        >
                            {i18n.t('settings.apply')}
                        </button>
                    </div>
                </div>

                <div class="flex flex-col md:flex-row md:items-center md:justify-between gap-3 border border-border-main rounded p-3">
                    <div class="text-text-main font-semibold">{i18n.t('settings.themeLabel')}</div>
                    <div class="flex items-center gap-2 flex-wrap">
                        <select
                            class="border border-border-main bg-bg-card text-text-main text-sm rounded px-3 py-2 min-w-[180px]"
                            bind:value={pendingTheme}
                        >
                            {#each availableThemes as t}
                                <option value={t.id}>{t.name}</option>
                            {/each}
                        </select>
                        <button
                            type="button"
                            class="px-3 py-2 bg-primary text-primary-fg rounded text-sm hover:bg-primary-hover"
                            onclick={applyTheme}
                        >
                            {i18n.t('settings.apply')}
                        </button>
                        <ThemeSelector />
                    </div>
                </div>
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
    </div>
</section>
