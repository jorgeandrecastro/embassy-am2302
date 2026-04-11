// Copyright (C) 2026 Jorge Andre Castro
//
// Ce programme est un logiciel libre : vous pouvez le redistribuer et/ou le modifier
// selon les termes de la Licence Publique Générale GNU telle que publiée par la
// Free Software Foundation, soit la version 2 de la licence, soit (à votre convention)
// n'importe quelle version ultérieure.

//! # signals
//!
//! Canal de communication entre la tâche de lecture [`crate::am2302_run`]
//! et le reste de l'application.
//!
//! ## Utilisation
//!
//! Le signal [`ENV_SIGNAL`] est un canal "last-value" : seule la dernière
//! mesure est conservée. Si personne ne consomme les données entre deux
//! lectures, l'ancienne valeur est écrasée.
//!
//! ```rust,ignore
//! use embassy_am2302::signals::ENV_SIGNAL;
//!
//! #[embassy_executor::task]
//! async fn afficher_env() {
//!     loop {
//!         let data = ENV_SIGNAL.wait().await;
//!         defmt::info!("Temp: {}°C  Humidité: {}%", data.temp, data.hum);
//!     }
//! }
//! ```

use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

/// Données environnementales lues depuis le capteur AM2302.
///
/// Les valeurs sont exprimées en unités physiques directement exploitables,
/// après décodage du format binaire DHT22 et division par 10.
///
/// # Champs
///
/// * `temp` — température en degrés Celsius (négatif possible)
/// * `hum`  — humidité relative en pourcentage `[0.0, 100.0]`
///
/// # Exemple
///
/// ```rust,ignore
/// let data = ENV_SIGNAL.wait().await;
/// assert!(data.hum >= 0.0 && data.hum <= 100.0);
/// ```
#[derive(Clone, Copy)]
pub struct EnvData {
    /// Température en °C. Peut être négative (bit de signe du DHT22).
    pub temp: f32,
    /// Humidité relative en %, dans la plage `[0.0, 100.0]`.
    pub hum: f32,
}

/// Signal global portant la dernière mesure publiée par [`crate::am2302_run`].
///
/// Utilise un mutex section critique (`CriticalSectionRawMutex`),
/// compatible avec les environnements sans OS (bare-metal, `no_std`).
///
/// # Comportement
///
/// - `signal()` — publie une nouvelle mesure (écrase la précédente si non lue)
/// - `wait()`   — attend de manière asynchrone la prochaine mesure disponible
pub static ENV_SIGNAL: Signal<CriticalSectionRawMutex, EnvData> = Signal::new();