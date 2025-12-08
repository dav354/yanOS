<script>
    import { auth } from '$lib/auth.svelte.js';

    let username = $state('');
    let password = $state('');

    async function handleLogin(event) {
        event.preventDefault();
        await auth.login(username, password);
    }
</script>

<div class="p-4">
    <h1 class="text-2xl font-bold mb-4">zOS Management</h1>

    {#if auth.isAuthenticated}
        <div class="bg-green-100 p-4 rounded">
            <p>Welcome, <strong>{auth.user}</strong>!</p>
        </div>
    {:else}
        <form onsubmit={handleLogin} class="bg-gray-100 p-4 rounded max-w-sm">
            <div class="mb-4">
                <label class="block text-gray-700 text-sm font-bold mb-2" for="username">
                    Username
                </label>
                <input class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 leading-tight focus:outline-none focus:shadow-outline" id="username" type="text" bind:value={username}>
            </div>
            <div class="mb-6">
                <label class="block text-gray-700 text-sm font-bold mb-2" for="password">
                    Password
                </label>
                <input class="shadow appearance-none border rounded w-full py-2 px-3 text-gray-700 mb-3 leading-tight focus:outline-none focus:shadow-outline" id="password" type="password" bind:value={password}>
            </div>
            <div class="flex items-center justify-between">
                <button class="bg-blue-500 hover:bg-blue-700 text-white font-bold py-2 px-4 rounded focus:outline-none focus:shadow-outline" type="submit">
                    Sign In
                </button>
            </div>
        </form>
    {/if}
</div>