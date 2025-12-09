<script>
    import '../app.css';
    import { auth } from '$lib/auth.svelte.js';
    import { page } from '$app/stores';
    import { goto } from '$app/navigation';
    import Sidebar from '$lib/components/Sidebar.svelte';
    import LogPanel from '$lib/components/LogPanel.svelte';

    let { children } = $props();

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
</script>

{#if !auth.isInitialized}
    <!-- Loading Screen -->
    <div class="flex h-screen w-screen items-center justify-center bg-gray-100">
        <div class="text-gray-500">Loading zOS...</div>
    </div>
{:else if $page.url.pathname === '/login'}
    {@render children?.()}
{:else}
    <!-- Main App Layout -->
    <div class="flex h-screen w-screen bg-bg-main text-text-main overflow-hidden font-sans transition-colors duration-200">
        <!-- Left Sidebar -->
        <Sidebar />

        <!-- Right Content Area -->
        <div class="flex-1 flex flex-col min-w-0 bg-bg-main">
            <!-- Top Bar (Optional, can be just a spacer or breadcrumbs) -->
            <!-- <header class="bg-white shadow h-12 flex items-center px-4"> ... </header> -->

            <!-- Main Page Content (Scrollable) -->
            <main class="flex-1 overflow-y-auto p-4 relative">
                {#if auth.isAuthenticated}
                    {@render children?.()}
                {:else}
                     <!-- Protected Route Guard -->
                     <div class="flex flex-col items-center justify-center h-full">
                        <h2 class="text-xl font-bold text-text-main mb-4">Access Denied</h2>
                        <p class="mb-4 text-text-muted">You must be logged in to view this page.</p>
                        <a href="/login" class="bg-primary hover:bg-primary-hover text-primary-fg px-4 py-2 rounded">Go to Login</a>
                     </div>
                {/if}
            </main>

            <!-- Bottom Log Panel -->
            <LogPanel />
        </div>
    </div>
{/if}
