<script>
    import { auth } from '$lib/auth.svelte.js';
    import { goto } from '$app/navigation';

    let username = $state('');
    let password = $state('');
    let error = $state(null);
    let isLoading = $state(false);

    async function handleLogin(event) {
        event.preventDefault();
        isLoading = true;
        error = null;
        
        try {
            const success = await auth.login(username, password);
            if (success) {
                if (typeof window !== 'undefined') {
                    window.location.href = '/';
                } else {
                    goto('/');
                }
            } else {
                error = 'Invalid credentials';
            }
        } catch (e) {
            error = 'Login error';
        } finally {
            isLoading = false;
        }
    }
</script>

<div class="min-h-screen flex items-center justify-center bg-bg-main">
    <div class="bg-bg-card p-8 rounded shadow-md w-full max-w-sm border border-border-main">
        <h1 class="text-2xl font-bold mb-6 text-center text-text-main">yanOS Login</h1>
        
        {#if error}
            <div class="bg-red-100 border border-red-400 text-red-700 px-4 py-3 rounded mb-4 text-sm">
                {error}
            </div>
        {/if}

        <form onsubmit={handleLogin} class="space-y-4">
            <div>
                <label class="block text-text-muted text-sm font-bold mb-2" for="username">
                    Username
                </label>
                <input 
                    class="shadow appearance-none border border-border-main rounded w-full py-2 px-3 bg-bg-input text-text-main leading-tight focus:outline-none focus:shadow-outline focus:ring-2 focus:ring-primary" 
                    id="username" 
                    type="text" 
                    bind:value={username}
                    placeholder="root"
                    disabled={isLoading}
                >
            </div>
            <div>
                <label class="block text-text-muted text-sm font-bold mb-2" for="password">
                    Password
                </label>
                <input 
                    class="shadow appearance-none border border-border-main rounded w-full py-2 px-3 bg-bg-input text-text-main mb-3 leading-tight focus:outline-none focus:shadow-outline focus:ring-2 focus:ring-primary" 
                    id="password" 
                    type="password" 
                    bind:value={password}
                    disabled={isLoading}
                >
            </div>
            <div class="flex items-center justify-between">
                <button 
                    class="w-full bg-primary hover:bg-primary-hover text-primary-fg font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline disabled:opacity-50" 
                    type="submit"
                    disabled={isLoading}
                >
                    {isLoading ? 'Signing In...' : 'Sign In'}
                </button>
            </div>
        </form>
    </div>
</div>
