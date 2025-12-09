import { browser } from '$app/environment';

const translations = {
    en: {
        nav: {
            dashboard: 'Dashboard',
            network: 'Network',
            terminal: 'Terminal',
            packages: 'Packages',
            logs: 'Logs',
            settings: 'Settings',
            logout: 'Logout',
            confirmLogout: 'Do you really want to log out?',
            cancel: 'Cancel',
        },
        sidebar: {
            systemOnline: 'System: Online',
            language: 'Language',
        },
        logs: {
            title: 'System Logs',
            subtitle: 'Live events in plain text, filterable and sortable.',
            unauth: 'Please log in to view logs.',
            none: 'No events received yet.',
            filterAll: 'All levels',
            filterError: 'Error',
            filterWarn: 'Warn',
            filterInfo: 'Info',
            sortTime: 'Newest first',
            sortLevel: 'Sort by level',
        },
        settings: {
            title: 'Settings',
            subtitle: 'Choose your theme and interface language.',
            theme: 'Theme',
            themeSubtitle: 'Pick a look for zOS.',
            themeActive: 'Active',
            themeActivate: 'Click to activate',
            language: 'Language',
            uiSection: 'User Interface',
            languageLabel: 'Language',
            themeLabel: 'Theme',
            apply: 'Apply',
            notifications: 'Notifications',
            integrations: 'Integrations',
            about: 'About',
            placeholder: 'Coming soon.',
            confirmLeave: 'You have unsaved changes. Save before leaving?',
        },
        network: {
            title: 'Network Interfaces',
            refresh: 'Refresh',
            loading: 'Loading interfaces...',
            empty: 'No interfaces found.',
            via: 'Via dladm/ipadm',
        },
        packages: {
            title: 'Installed Packages',
            refresh: 'Refresh',
            loading: 'Loading package list...',
            empty: 'No packages found (or unable to fetch).',
        },
        tasks: {
            title: 'Tasks',
            subtitle: 'Scrubs, replication, and scheduled jobs.',
            none: 'No active tasks.',
        },
    },
    de: {
        nav: {
            dashboard: 'Dashboard',
            network: 'Netzwerk',
            terminal: 'Terminal',
            packages: 'Pakete',
            logs: 'Logs',
            settings: 'Einstellungen',
            logout: 'Abmelden',
            confirmLogout: 'Möchtest du dich wirklich abmelden?',
            cancel: 'Abbrechen',
        },
        sidebar: {
            systemOnline: 'System: Online',
            language: 'Sprache',
        },
        logs: {
            title: 'System-Logs',
            subtitle: 'Live-Events als Klartext, filter- und sortierbar.',
            unauth: 'Bitte erst anmelden, um Logs zu sehen.',
            none: 'Noch keine Events empfangen.',
            filterAll: 'Alle Level',
            filterError: 'Error',
            filterWarn: 'Warn',
            filterInfo: 'Info',
            sortTime: 'Neueste zuerst',
            sortLevel: 'Nach Level sortieren',
        },
        settings: {
            title: 'Einstellungen',
            subtitle: 'Theme und Sprache wählen.',
            theme: 'Theme',
            themeSubtitle: 'Look für zOS wählen.',
            themeActive: 'Aktiv',
            themeActivate: 'Zum Aktivieren klicken',
            language: 'Sprache',
            uiSection: 'Benutzeroberfläche',
            languageLabel: 'Sprache',
            themeLabel: 'Theme',
            apply: 'Übernehmen',
            notifications: 'Benachrichtigungen',
            integrations: 'Integrationen',
            about: 'Über',
            placeholder: 'In Kürze verfügbar.',
            confirmLeave: 'Du hast ungespeicherte Änderungen. Vor dem Verlassen speichern?',
        },
        network: {
            title: 'Netzwerk-Interfaces',
            refresh: 'Neu laden',
            loading: 'Lade Interfaces...',
            empty: 'Keine Interfaces gefunden.',
            via: 'Über dladm/ipadm',
        },
        packages: {
            title: 'Installierte Pakete',
            refresh: 'Neu laden',
            loading: 'Lade Paketliste...',
            empty: 'Keine Pakete gefunden (oder Fehler beim Abruf).',
        },
        tasks: {
            title: 'Tasks',
            subtitle: 'Scrubs, Replikation und geplante Jobs.',
            none: 'Keine aktiven Tasks.',
        },
    },
};

class I18nStore {
    current = $state('de');

    constructor() {
        if (browser) {
            const saved = localStorage.getItem('zos-lang');
            if (saved && translations[saved]) {
                this.current = saved;
            }
        }
    }

    setLang(lang) {
        if (!translations[lang]) return;
        this.current = lang;
        if (browser) {
            localStorage.setItem('zos-lang', lang);
        }
    }

    t(key, fallback) {
        const parts = key.split('.');
        let value = translations[this.current];
        for (const p of parts) {
            value = value?.[p];
        }
        return value ?? fallback ?? key;
    }
}

export const i18n = new I18nStore();
export const availableLangs = [
    { id: 'de', name: 'Deutsch' },
    { id: 'en', name: 'English' },
];
