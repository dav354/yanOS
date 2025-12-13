<script>
    import { page } from '$app/stores';
    import { i18n, availableLangs } from '$lib/i18n.svelte.js';
    
    // Simple helper to check active state
    function isActive(path) {
        return $page.url.pathname === path;
    }

    let lang = $state(i18n.current);

    const links = [
        { key: 'nav.dashboard', href: '/' },
        { key: 'nav.network', href: '/network' },
        { key: 'nav.terminal', href: '/terminal' },
        { key: 'nav.packages', href: '/packages' },
        { key: 'nav.logs', href: '/logs' },
        { key: 'nav.settings', href: '/settings' }
    ];

    const settingsSections = [
        { id: 'ui', key: 'settings.uiSection' },
        { id: 'telemetry', key: 'settings.telemetry' },
        { id: 'notifications', key: 'settings.notifications' },
        { id: 'about', key: 'settings.about' },
    ];

    let currentHash = $derived($page.url.hash || '');
    let onSettings = $derived(($page.url.pathname || '').startsWith('/settings'));

    function changeLang(event) {
        const newLang = event.target.value;
        i18n.setLang(newLang);
        lang = newLang;
    }

    let showLogoutConfirm = $state(false);

    function requestLogout(event) {
        event.preventDefault();
        showLogoutConfirm = true;
    }

    function doLogout() {
        showLogoutConfirm = false;
        fetch('/api/v1/logout', { method: 'POST', credentials: 'include' }).finally(() => {
            window.location.href = '/login';
        });
    }
</script>

<aside class="w-64 bg-bg-sidebar text-text-sidebar flex flex-col h-full border-r border-border-main">
    <div class="p-4 border-b border-border-main bg-bg-sidebar">
        <h1 class="text-xl font-bold tracking-wider">yanOS <span class="text-xs text-text-sidebar-muted font-normal">v0.1</span></h1>
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
                        {i18n.t(link.key)}
                    </a>
                    {#if onSettings && link.href === '/settings'}
                        <ul class="mt-1 space-y-1 pl-4 border-l border-border-main/50">
                            {#each settingsSections as section}
                                <li>
                                    <a
                                        href={`/settings#${section.id}`}
                                        class={"flex items-center px-3 py-1.5 text-xs transition-colors duration-150 rounded "
                                            + (currentHash === `#${section.id}`
                                                ? 'bg-primary text-primary-fg'
                                                : 'text-text-sidebar-muted hover:bg-white/5 hover:text-text-sidebar')}
                                    >
                                        {i18n.t(section.key)}
                                    </a>
                                </li>
                            {/each}
                        </ul>
                    {/if}
                </li>
            {/each}
        </ul>
    </nav>

    <div class="p-4 bg-bg-sidebar">
        <button
            onclick={requestLogout}
            class="w-full text-left px-4 py-2 text-sm font-medium rounded border border-border-main text-red-400 hover:bg-red-500/10 hover:text-red-200 transition-colors"
        >
            {i18n.t('nav.logout')}
        </button>
    </div>

    <div class="p-4 border-t border-border-main bg-bg-sidebar space-y-3">
        <div class="flex items-center justify-between">
            <div class="text-xs text-text-sidebar-muted">
                <p>{i18n.t('sidebar.systemOnline')}</p>
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
    </div>

    {#if showLogoutConfirm}
        <div class="fixed inset-0 bg-black/40 flex items-center justify-center z-50">
            <div class="bg-bg-card border border-border-main rounded shadow-lg max-w-sm w-full mx-4 p-4">
                <h3 class="text-lg font-semibold text-text-main mb-2">{i18n.t('nav.logout')}</h3>
                <p class="text-text-muted text-sm mb-4">{i18n.t('nav.confirmLogout')}</p>
                <div class="flex justify-end gap-2">
                    <button
                        class="px-3 py-2 text-sm rounded border border-border-main text-text-main hover:bg-bg-main"
                        onclick={() => showLogoutConfirm = false}
                        type="button"
                    >
                        {i18n.t('nav.cancel')}
                    </button>
                    <button
                        class="px-3 py-2 text-sm rounded bg-red-500 text-white hover:bg-red-600"
                        onclick={doLogout}
                        type="button"
                    >
                        {i18n.t('nav.logout')}
                    </button>
                </div>
            </div>
        </div>
    {/if}
</aside>
