// Copyright (C) 2026 Jorge Andre Castro
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

#![no_std]

//! Driver async no_std pour le capteur de température et d'humidité AM2302 (DHT22).
//! Compatible avec toutes les cartes et tous les exécuteurs via `embedded-hal`.
//!
//! ## Caractéristiques
//!
//! - `#![no_std]` — aucune dépendance à la bibliothèque standard
//! - Zéro dépendance Embassy — fonctionne avec n'importe quel exécuteur async
//! - Compatible RP2040, STM32, nRF, ESP32… via les traits `embedded-hal`
//! - Protocole 1-Wire implémenté bit à bit
//! - Vérification de la somme de contrôle (checksum) intégrée
//! - Support des températures négatives
//!
//! ## Protocole de communication
//!
//! Le DHT22 utilise un protocole 1-Wire propriétaire :
//!
//! ```text
//! Maître  ──── 20ms bas ────┐
//! Capteur                   └── 80µs bas ── 80µs haut ──┐
//! Bit 0   : 50µs bas + ~28µs haut                        │  × 40 bits
//! Bit 1   : 50µs bas + ~70µs haut                        │
//! ```
//!
//! Un signal haut `> 40µs` est interprété comme un bit `1`, sinon bit `0`.
//!
//! ## Format des données
//!
//! Les 40 bits reçus sont répartis en 5 octets :
//!
//! ```text
//! [0] humidité    (partie entière)
//! [1] humidité    (partie décimale)
//! [2] température (partie entière, bit 7 = signe négatif)
//! [3] température (partie décimale)
//! [4] checksum    = [0] + [1] + [2] + [3]
//! ```
//!
//! ## Exemple — Embassy RP2040
//!
//! ```rust,ignore
//! use embassy_rp::gpio::Flex;
//! use embassy_time::Delay;
//! use embassy_am2302::am2302_read;
//!
//! #[embassy_executor::task]
//! async fn sensor_task(mut pin: Flex<'static>) {
//!     let mut delay = Delay;
//!     loop {
//!         match am2302_read(&mut pin, &mut delay).await {
//!             Ok(data) => ENV_SIGNAL.signal(data),
//!             Err(_)   => {}
//!         }
//!         delay.delay_ms(3000).await;
//!     }
//! }
//! ```

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::delay::DelayNs;

/// Données environnementales lues depuis le capteur AM2302.
///
/// Les valeurs sont exprimées en unités physiques directement exploitables,
/// après décodage du format binaire DHT22 et division par 10.
///
/// # Champs
///
/// * `temp` — température en degrés Celsius (négatif possible)
/// * `hum`  — humidité relative en pourcentage `[0.0, 100.0]`
#[derive(Clone, Copy, Debug)]
pub struct EnvData {
    /// Température en °C. Peut être négative (bit de signe du DHT22).
    pub temp: f32,
    /// Humidité relative en %, dans la plage `[0.0, 100.0]`.
    pub hum: f32,
}

/// Erreurs possibles lors de la lecture du capteur AM2302.
#[derive(Debug, PartialEq)]
pub enum Am2302Error<E> {
    /// Timeout pendant le handshake ou la lecture des bits.
    Timeout,
    /// Checksum invalide — données corrompues ou transmission incomplète.
    ChecksumMismatch,
    /// Erreur matérielle retournée par le HAL GPIO.
    Gpio(E),
}

/// Lit une seule mesure depuis le capteur AM2302.
///
/// # Arguments
///
/// * `pin`   — broche GPIO implémentant [`InputPin`] + [`OutputPin`]
/// * `delay` — implémentation async de [`DelayNs`] fournie par le HAL
///
/// # Retour
///
/// `Ok(EnvData)` si la lecture et le checksum sont valides,
/// `Err(Am2302Error)` sinon.
///
/// # Exemple
///
/// ```rust,ignore
/// match am2302_read(&mut pin, &mut delay).await {
///     Ok(data)                           => defmt::info!("{}°C  {}%", data.temp, data.hum),
///     Err(Am2302Error::ChecksumMismatch) => defmt::warn!("Données corrompues"),
///     Err(Am2302Error::Timeout)          => defmt::warn!("Capteur ne répond pas"),
///     Err(Am2302Error::Gpio(_))          => defmt::error!("Erreur GPIO"),
/// }
/// ```
pub async fn am2302_read<P, E>(
    pin: &mut P,
    delay: &mut impl DelayNs,
) -> Result<EnvData, Am2302Error<E>>
where
    P: InputPin<Error = E> + OutputPin<Error = E>,
{
    // 1. SIGNAL DE START
    // La spec DHT22 exige minimum 1ms ; on utilise 20ms pour la robustesse.
    pin.set_low().map_err(Am2302Error::Gpio)?;
    delay.delay_ms(20).await;
    pin.set_high().map_err(Am2302Error::Gpio)?;

    // 2. ATTENTE DU HANDSHAKE
    // Séquence : attente bas (80µs) → haut (80µs) → fin handshake
    let mut timeout = 0u32;

    while pin.is_high().map_err(Am2302Error::Gpio)? {
        timeout += 1;
        if timeout > 10000 { return Err(Am2302Error::Timeout); }
    }
    timeout = 0;
    while pin.is_low().map_err(Am2302Error::Gpio)? {
        timeout += 1;
        if timeout > 10000 { return Err(Am2302Error::Timeout); }
    }
    timeout = 0;
    while pin.is_high().map_err(Am2302Error::Gpio)? {
        timeout += 1;
        if timeout > 10000 { return Err(Am2302Error::Timeout); }
    }

    // 3. LECTURE DES 40 BITS
    // Chaque bit est précédé d'un signal bas de ~50µs.
    // La durée du signal haut détermine la valeur du bit :
    //   < 40µs → bit 0  |  > 40µs → bit 1
    // On compte les itérations de boucle comme proxy temporel —
    // aucun timer hardware requis.
    let mut data = [0u8; 5];
    for i in 0..40 {
        timeout = 0;
        while pin.is_low().map_err(Am2302Error::Gpio)? {
            timeout += 1;
            if timeout > 10000 { return Err(Am2302Error::Timeout); }
        }

        // Mesure de la durée du signal haut par comptage de boucles.
        // Seuil empirique : ~100 itérations séparent un bit 0 d'un bit 1
        // sur un Cortex-M0+ à 125MHz sans optimisation.
        let mut high_count = 0u32;
        while pin.is_high().map_err(Am2302Error::Gpio)? {
            high_count += 1;
            if high_count > 200 { break; }
        }

        if high_count > 40 {
            data[i / 8] |= 1 << (7 - (i % 8));
        }
    }

    // 4. VALIDATION DU CHECKSUM
    // Le checksum est la somme tronquée des 4 premiers octets.
    let checksum = data[0]
        .wrapping_add(data[1])
        .wrapping_add(data[2])
        .wrapping_add(data[3]);

    if data[4] != checksum {
        return Err(Am2302Error::ChecksumMismatch);
    }

    // 5. DÉCODAGE
    let hum = (((data[0] as u16) << 8) | data[1] as u16) as f32 / 10.0;

    // Bit 7 de data[2] = indicateur de signe négatif
    let mut temp = ((((data[2] & 0x7F) as u16) << 8) | data[3] as u16) as f32 / 10.0;
    if (data[2] & 0x80) != 0 {
        temp *= -1.0;
    }

    Ok(EnvData { temp, hum })
}