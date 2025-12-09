<script>
    import { theme, availableThemes } from '$lib/theme.svelte.js';

    let isOpen = $state(false);

    function toggle() {
        isOpen = !isOpen;
    }

    function select(id) {
        theme.setTheme(id);
        isOpen = false;
    }
</script>

<div class="relative">
    <button 
        onclick={toggle} 
        class="flex items-center space-x-2 px-3 py-2 rounded hover:bg-white/10 text-sm font-medium transition-colors"
    >
        <span class="inline-block h-2 w-2 rounded-full bg-primary"></span>
        <span>Theme</span>
    </button>

    {#if isOpen}
        <div class="absolute bottom-full left-0 mb-2 w-48 bg-bg-card border border-border-main rounded shadow-lg overflow-hidden z-50">
            {#each availableThemes as t}
                <button 
                    onclick={() => select(t.id)}
                    class="block w-full text-left px-4 py-2 text-sm text-text-main hover:bg-bg-main hover:text-primary transition-colors
                    {theme.current === t.id ? 'font-bold text-primary' : ''}"
                >
                    {t.name}
                </button>
            {/each}
        </div>
    {/if}
</div>
