import { browser } from '$app/environment';

const translations = {
    en: {
        nav: {
            dashboard: 'Dashboard',
            storage: 'Storage',
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
            liveConnected: 'Live stream connected',
            liveDisconnected: 'Reconnecting log stream...',
            loadError: 'Unable to load logs',
            streamError: 'Log stream disconnected',
        },
        settings: {
            title: 'Settings',
            subtitle: 'Choose your theme and interface language.',
            theme: 'Theme',
            themeSubtitle: 'Pick a look for yanOS.',
            themeActive: 'Active',
            themeActivate: 'Click to activate',
            language: 'Language',
            uiSection: 'User Interface',
            languageLabel: 'Language',
            themeLabel: 'Theme',
            apply: 'Apply',
            telemetry: 'Telemetry',
            telemetrySubtitle: 'Configure OTLP export to your collector.',
            otlpEndpoint: 'OTLP endpoint',
            otlpPlaceholder: 'http://collector:4317',
            telemetryDisabled: 'Telemetry is disabled when no endpoint is set.',
            telemetryTest: 'Test',
            telemetryTestSuccess: 'Endpoint reachable',
            telemetryTestFail: 'Endpoint unreachable',
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
            updatesAvailable: 'Updates Available',
            applyAll: 'Apply All Updates',
            checkUpdates: 'Check Updates',
            updateAction: 'Update',
            updateAvailableStatus: 'Update Available',
            statusHeader: 'IPS Status Codes (XYZ):\n1. Pos (State): i=installed, -=not installed\n2. Pos (Meta): m=manifest defined, -=none\n3. Pos (Freeze): f=frozen, -=active\n4. Pos (Obsolete): o=obsolete, r=renamed, -=current',
        },
        tasks: {
            title: 'Tasks',
            subtitle: 'Scrubs, replication, and scheduled jobs.',
            none: 'No active tasks.',
        },
        storage: {
            title: 'Storage Pools',
            subtitle: 'ZFS pools and datasets.',
            refresh: 'Refresh',
            loading: 'Loading pools...',
            empty: 'No pools found.',
            pool: 'Pool',
            health: 'Health',
            capacity: 'Capacity',
            used: 'Used',
            free: 'Free',
            fragmentation: 'Frag',
            datasets: 'Datasets',
            viewDatasets: 'View Datasets',
            online: 'ONLINE',
            degraded: 'DEGRADED',
            faulted: 'FAULTED',
            offline: 'OFFLINE',
        },
    },
    de: {
        nav: {
            dashboard: 'Dashboard',
            storage: 'Speicher',
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
            liveConnected: 'Live-Stream verbunden',
            liveDisconnected: 'Log-Stream wird neu verbunden...',
            loadError: 'Logs konnten nicht geladen werden',
            streamError: 'Log-Stream getrennt',
        },
        settings: {
            title: 'Einstellungen',
            subtitle: 'Theme und Sprache wählen.',
            theme: 'Theme',
            themeSubtitle: 'Look für yanOS wählen.',
            themeActive: 'Aktiv',
            themeActivate: 'Zum Aktivieren klicken',
            language: 'Sprache',
            uiSection: 'Benutzeroberfläche',
            languageLabel: 'Sprache',
            themeLabel: 'Theme',
            apply: 'Übernehmen',
            telemetry: 'Telemetry',
            telemetrySubtitle: 'OTLP Export zum Collector konfigurieren.',
            otlpEndpoint: 'OTLP-Endpunkt',
            otlpPlaceholder: 'http://collector:4317',
            telemetryDisabled: 'Telemetry ist deaktiviert, wenn kein Endpunkt gesetzt ist.',
            telemetryTest: 'Testen',
            telemetryTestSuccess: 'Endpunkt erreichbar',
            telemetryTestFail: 'Endpunkt nicht erreichbar',
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
            updatesAvailable: 'Verfügbare Updates',
            applyAll: 'Alle aktualisieren',
            checkUpdates: 'Nach Updates suchen',
            updateAction: 'Aktualisieren',
            updateAvailableStatus: 'Update verfügbar',
            statusHeader: 'IPS Status Codes (XYZ):\n1. Pos (Status): i=installiert, -=nicht installiert\n2. Pos (Meta): m=Manifest definiert, -=keines\n3. Pos (Freeze): f=eingefroren, -=aktiv\n4. Pos (Veraltet): o=obsolete, r=umbenannt, -=aktuell',
        },
        tasks: {
            title: 'Tasks',
            subtitle: 'Scrubs, Replikation und geplante Jobs.',
            none: 'Keine aktiven Tasks.',
        },
        storage: {
            title: 'Speicher-Pools',
            subtitle: 'ZFS Pools und Datasets.',
            refresh: 'Neu laden',
            loading: 'Lade Pools...',
            empty: 'Keine Pools gefunden.',
            pool: 'Pool',
            health: 'Zustand',
            capacity: 'Kapazität',
            used: 'Belegt',
            free: 'Frei',
            fragmentation: 'Frag',
            datasets: 'Datasets',
            viewDatasets: 'Datasets anzeigen',
            online: 'ONLINE',
            degraded: 'DEGRADED',
            faulted: 'FAULTED',
            offline: 'OFFLINE',
        },
    },
};

class I18nStore {
    current = $state('de');

    constructor() {
        if (browser) {
            const saved = localStorage.getItem('yanos-lang');
            if (saved && translations[saved]) {
                this.current = saved;
            }
        }
    }

    setLang(lang) {
        if (!translations[lang]) return;
        this.current = lang;
        if (browser) {
            localStorage.setItem('yanos-lang', lang);
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
