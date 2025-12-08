import { browser } from '$app/environment';

class AuthStore {
    user = $state(null);
    csrfToken = $state(null);
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
            }
        } catch (e) {
            console.error('Failed to init auth', e);
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
            this.user = username;
            return true;
        } else {
            console.error('Login failed');
            return false;
        }
    }
}

export const auth = new AuthStore();
