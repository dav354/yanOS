<script>
    import { page } from '$app/stores';
    import ThemeSelector from '$lib/components/ThemeSelector.svelte';
    
    // Simple helper to check active state
    function isActive(path) {
        return $page.url.pathname === path;
    }

    const links = [
        { name: 'Dashboard', href: '/' },
        { name: 'Network', href: '/network' },
        { name: 'Terminal', href: '/terminal' },
        { name: 'Packages', href: '/packages' }
    ];
</script>

<aside class="w-64 bg-bg-sidebar text-text-sidebar flex flex-col h-full border-r border-border-main">
    <div class="p-4 border-b border-border-main bg-bg-sidebar">
        <h1 class="text-xl font-bold tracking-wider">zOS <span class="text-xs text-text-sidebar-muted font-normal">v0.1</span></h1>
    </div>
    
    <nav class="flex-1 overflow-y-auto py-4">
        <ul class="space-y-1">
            {#each links as link}
                <li>
                    <a 
                        href={link.href} 
                        class="flex items-center px-4 py-2 text-sm font-medium transition-colors duration-150
                        {isActive(link.href) ? 'bg-primary text-primary-fg' : 'text-text-sidebar-muted hover:bg-white/5 hover:text-text-sidebar'}"
                    >
                        {link.name}
                    </a>
                </li>
            {/each}
        </ul>
    </nav>
    
    <div class="p-4 border-t border-border-main bg-bg-sidebar">
        <ThemeSelector />
        <div class="mt-2 text-xs text-text-sidebar-muted">
            <p>System: Online</p>
        </div>
    </div>
</aside>
