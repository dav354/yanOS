import { browser } from '$app/environment';

class ThemeStore {
    current = $state('default'); // default, gruvbox, rose-pine, one-dark, nord

    constructor() {
        if (browser) {
            const saved = localStorage.getItem('yanos-theme');
            if (saved) {
                this.setTheme(saved);
            }
        }
    }

    setTheme(name) {
        this.current = name;
        if (browser) {
            localStorage.setItem('yanos-theme', name);
            document.documentElement.setAttribute('data-theme', name);
        }
    }
}

export const theme = new ThemeStore();

export const availableThemes = [
    { id: 'default', name: 'Default (Light)' },
    { id: 'gruvbox', name: 'Gruvbox Dark' },
    { id: 'rose-pine', name: 'Rosé Pine' },
    { id: 'one-dark', name: 'One Dark' },
    { id: 'nord', name: 'Nord' },
];
