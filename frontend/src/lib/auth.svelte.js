import { browser } from '$app/environment';

class AuthStore {
    user = $state(null);
    csrfToken = $state(null);
    isInitialized = $state(false);
    isAuthenticated = $derived(!!this.user);

    constructor() {
        if (browser) {
            this.init();
        }
    }

    async init() {
        try {
            const res = await fetch('/api/v1/status');
            if (res.ok) {
                const data = await res.json();
                this.csrfToken = data.csrf_token;
                this.user = data.user ?? null;
            }
        } catch (e) {
            console.error('Failed to init auth', e);
        } finally {
            this.isInitialized = true;
        }
    }

    async login(username, password) {
        if (!this.csrfToken) {
            await this.init();
        }

        const res = await fetch('/api/login', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-CSRF-TOKEN': this.csrfToken
            },
            body: JSON.stringify({ username, password })
        });

        if (res.ok) {
            await this.init();
            this.user = username;
            return true;
        } else {
            this.user = null;
            console.error('Login failed');
            return false;
        }
    }
}

export const auth = new AuthStore();
