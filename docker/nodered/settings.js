/**
 * Node-RED settings — Daly-BMS-Rust
 *
 * Personnalisations clés :
 *  - contextStorage : active le store "file" (localfilesystem) en plus du store mémoire
 *    → les globals de production (pvinv_baseline, total_yield_today, etc.) survivent
 *      aux redémarrages de Node-RED / Pi5.
 *
 * Les flux, credentials et secrets sont dans le volume Docker nodered-data (/data).
 * Ce fichier ne contient aucun secret.
 */

module.exports = {

    // Port d'écoute (override par variable d'environnement PORT si nécessaire)
    uiPort: process.env.PORT || 1880,
    uiHost: "0.0.0.0",

    // ── Contexte persistant ───────────────────────────────────────────────────
    // "default" = mémoire rapide pour variables éphémères (irradiance_wm2, outdoor_temp…)
    // "file"    = disque pour variables de production (pvinv_baseline, total_yield_today…)
    //             → survit aux redémarrages Docker/Pi5
    contextStorage: {
        default: {
            module: "memory"
        },
        file: {
            module: "localfilesystem"
        }
    },

    // ── Modules externes dans les Function nodes ──────────────────────────────
    functionExternalModules: true,

    // ── Logs ─────────────────────────────────────────────────────────────────
    logging: {
        console: {
            level: "info",
            metrics: false,
            audit: false
        }
    },

    // ── Éditeur ──────────────────────────────────────────────────────────────
    editorTheme: {
        projects: {
            enabled: false
        }
    },

    // Délai de reconnexion MQTT/serial (ms)
    mqttReconnectTime: 15000,
    serialReconnectTime: 15000,

    // Longueur max des messages de debug
    debugMaxLength: 1000,

    exportGlobalContextKeys: false
};
