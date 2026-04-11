// Copyright (C) 2026 Jorge Andre Castro
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

#![no_std]

//! Driver async no_std pour le capteur de température et d'humidité AM2302 (DHT22).
//! Compatible avec toutes les cartes supportées par Embassy via `embedded-hal`.

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::delay::DelayNs;
use embassy_time::Instant;
use signals::{EnvData, ENV_SIGNAL};

pub mod signals;

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
/// Fonction bas niveau. Préférer [`am2302_run`] pour une boucle de lecture continue.
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
///     Ok(data)                          => defmt::info!("{}°C  {}%", data.temp, data.hum),
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
    let mut data = [0u8; 5];
    for i in 0..40 {
        timeout = 0;
        while pin.is_low().map_err(Am2302Error::Gpio)? {
            timeout += 1;
            if timeout > 10000 { return Err(Am2302Error::Timeout); }
        }

        let start = Instant::now();
        while pin.is_high().map_err(Am2302Error::Gpio)? {
            if start.elapsed().as_micros() > 100 { break; }
        }

        if start.elapsed().as_micros() > 40 {
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

/// Boucle de lecture continue du capteur AM2302.
///
/// Lit le capteur toutes les 3 secondes et publie les données valides
/// via [`signals::ENV_SIGNAL`]. Les erreurs sont silencieusement ignorées.
///
/// Cette fonction ne retourne jamais — elle est conçue pour être exécutée
/// dans une tâche Embassy dédiée. `#[embassy_executor::task]` n'acceptant
/// pas les génériques, l'utilisateur crée une fine tâche d'encapsulation.
///
/// # Arguments
///
/// * `pin`   — broche GPIO implémentant [`InputPin`] + [`OutputPin`]
/// * `delay` — implémentation async de [`DelayNs`] fournie par le HAL
///
/// # Exemple
///
/// ```rust,ignore
/// // RP2040
/// #[embassy_executor::task]
/// async fn sensor_task(pin: embassy_rp::gpio::Flex<'static>, delay: embassy_time::Delay) {
///     am2302_run(pin, delay).await;
/// }
///
/// // STM32
/// #[embassy_executor::task]
/// async fn sensor_task(pin: embassy_stm32::gpio::Flex<'static>, delay: embassy_time::Delay) {
///     am2302_run(pin, delay).await;
/// }
/// ```
pub async fn am2302_run<P, E>(mut pin: P, mut delay: impl DelayNs) -> !
where
    P: InputPin<Error = E> + OutputPin<Error = E>,
{
    loop {
        if let Ok(data) = am2302_read(&mut pin, &mut delay).await {
            ENV_SIGNAL.signal(data);
        }

        // Le DHT22 nécessite au minimum 2s entre deux mesures.
        // On attend 3s pour garantir la stabilité.
        delay.delay_ms(3000).await;
    }
}