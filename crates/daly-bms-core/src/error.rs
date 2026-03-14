//! Types d'erreurs pour daly-bms-core

use thiserror::Error;

/// Erreurs possibles lors de la communication avec un BMS Daly.
#[derive(Debug, Error)]
pub enum DalyError {
    /// Le port série n'a pas pu être ouvert ou est devenu inaccessible.
    #[error("Erreur port série : {0}")]
    Serial(#[from] tokio_serial::Error),

    /// Erreur d'entrée/sortie bas niveau.
    #[error("Erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    /// Aucune réponse reçue dans le délai imparti.
    #[error("Timeout en attente de réponse du BMS {bms_id:#04x} (cmd {cmd:#04x})")]
    Timeout { bms_id: u8, cmd: u8 },

    /// La trame reçue a un checksum invalide.
    #[error("Checksum invalide : attendu {expected:#04x}, reçu {actual:#04x}")]
    Checksum { expected: u8, actual: u8 },

    /// La trame reçue est trop courte ou mal formée.
    #[error("Trame invalide ({len} octets) : {reason}")]
    InvalidFrame { len: usize, reason: &'static str },

    /// L'adresse BMS dans la réponse ne correspond pas à celle demandée.
    #[error("Adresse BMS inattendue : attendu {expected:#04x}, reçu {actual:#04x}")]
    UnexpectedAddress { expected: u8, actual: u8 },

    /// L'octet Start Flag n'est pas 0xA5.
    #[error("Start flag invalide : {0:#04x} (attendu 0xA5)")]
    InvalidStartFlag(u8),

    /// Le Data ID dans la réponse ne correspond pas à la requête.
    #[error("Data ID inattendu : attendu {expected:#04x}, reçu {actual:#04x}")]
    UnexpectedDataId { expected: u8, actual: u8 },

    /// Le BMS à l'adresse donnée n'a pas répondu à la découverte.
    #[error("BMS {0:#04x} non détecté sur le bus")]
    NotFound(u8),

    /// Commande d'écriture refusée (mode read-only activé).
    #[error("Commande d'écriture refusée : mode read-only activé")]
    ReadOnly,

    /// La vérification post-écriture a échoué.
    #[error("Vérification post-écriture échouée pour la commande {cmd:#04x} sur BMS {bms_id:#04x}")]
    VerifyFailed { bms_id: u8, cmd: u8 },

    /// Erreur générique enveloppée.
    #[error(transparent)]
    Other(#[from] anyhow::Error),
}

/// Alias de résultat pratique.
pub type Result<T> = std::result::Result<T, DalyError>;
