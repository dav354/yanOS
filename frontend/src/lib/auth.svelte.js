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
            const res = await fetch('/api/v1/status', { credentials: 'include' });
            if (res.ok) {
                const data = await res.json();
                this.csrfToken = this.readCsrfFromCookie();
                this.user = data.user ?? null;
            }
        } catch (e) {
            console.error('Failed to init auth', e);
        } finally {
            this.isInitialized = true;
        }
    }

    readCsrfFromCookie() {
        if (!browser) return null;
        const match = document.cookie.match(/(?:^|;\s*)XSRF-TOKEN=([^;]+)/);
        return match ? decodeURIComponent(match[1]) : null;
    }

    async login(username, password) {
        if (!this.csrfToken) {
            await this.init();
        }

        const token = this.csrfToken ?? this.readCsrfFromCookie();

        const res = await fetch('/api/v1/login', {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                ...(token ? { 'X-CSRF-TOKEN': token } : {})
            },
            credentials: 'include',
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
