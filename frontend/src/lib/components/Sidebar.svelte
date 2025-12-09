<script>
    import { page } from '$app/stores';
    
    // Simple helper to check active state
    function isActive(path) {
        return $page.url.pathname === path;
    }

    const links = [
        { name: 'Dashboard', href: '/' },
        { name: 'Network', href: '/network' },
        { name: 'Terminal', href: '/terminal' },
        { name: 'Packages', href: '/packages' },
        { name: 'Logs', href: '/logs' },
        { name: 'Settings', href: '/settings' }
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
    
    <div class="p-4 border-t border-border-main bg-bg-sidebar flex items-center justify-between">
        <div class="text-xs text-text-sidebar-muted">
            <p>System: Online</p>
        </div>
        <a
            href="https://github.com"
            class="text-text-sidebar-muted hover:text-text-sidebar"
            aria-label="Project repository on GitHub"
            rel="noreferrer"
            target="_blank"
        >
            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5">
                <path d="M12 .5C5.65.5.5 5.65.5 12a11.5 11.5 0 0 0 7.87 10.94c.58.11.79-.25.79-.56l-.01-2c-3.2.69-3.88-1.54-3.88-1.54-.53-1.34-1.28-1.7-1.28-1.7-1.05-.72.08-.7.08-.7 1.16.08 1.77 1.19 1.77 1.19 1.04 1.78 2.74 1.27 3.41.97.11-.76.4-1.27.73-1.56-2.55-.29-5.23-1.27-5.23-5.67 0-1.25.45-2.28 1.19-3.08-.12-.29-.52-1.46.11-3.05 0 0 .97-.31 3.18 1.18.92-.26 1.9-.39 2.88-.4.98 0 1.96.14 2.88.4 2.2-1.49 3.17-1.18 3.17-1.18.63 1.59.23 2.76.11 3.05.74.8 1.18 1.83 1.18 3.08 0 4.41-2.69 5.38-5.25 5.66.41.36.78 1.09.78 2.21l-.01 3.27c0 .31.21.68.8.56A11.5 11.5 0 0 0 23.5 12C23.5 5.65 18.35.5 12 .5Z"/>
            </svg>
        </a>
    </div>
</aside>
