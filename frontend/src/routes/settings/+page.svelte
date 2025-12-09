<script>
    import ThemeSelector from '$lib/components/ThemeSelector.svelte';
    import { availableThemes, theme } from '$lib/theme.svelte.js';

    let current = $derived(theme.current);

    function choose(id) {
        theme.setTheme(id);
    }
</script>

<section class="p-6 space-y-6">
    <header>
        <h1 class="text-2xl font-bold text-text-main">Settings</h1>
        <p class="text-text-muted">Wähle dein Theme und passe die Oberfläche an.</p>
    </header>

    <div class="border border-border-main rounded bg-bg-card">
        <div class="flex items-center justify-between px-4 py-3 border-b border-border-main">
            <div>
                <h2 class="text-lg font-semibold text-text-main">Theme</h2>
                <p class="text-text-muted text-sm">Wähle einen Look für zOS.</p>
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
                    <div class="text-xs text-text-muted mt-1">{current === t.id ? 'Aktiv' : 'Klicken zum Aktivieren'}</div>
                </button>
            {/each}
        </div>
    </div>
</section>
