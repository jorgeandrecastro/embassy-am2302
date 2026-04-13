// Copyright (C) 2026 Jorge Andre Castro
// GPL-2.0-or-later

//! Signal global portant la dernière mesure publiée par [`crate::am2302_read`].
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

use crate::EnvData;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};

/// Signal global portant la dernière mesure publiée par [`crate::am2302_read`].
///
/// Utilise un mutex section critique (`CriticalSectionRawMutex`),
/// compatible avec les environnements sans OS (bare-metal, `no_std`).
///
/// # Comportement
///
/// - `signal()` — publie une nouvelle mesure (écrase la précédente si non lue)
/// - `wait()`   — attend de manière asynchrone la prochaine mesure disponible
pub static ENV_SIGNAL: Signal<CriticalSectionRawMutex, EnvData> = Signal::new();