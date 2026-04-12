// Copyright (C) 2026 Jorge Andre Castro
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 2 of the License, or
// (at your option) any later version.

#![no_std]

//! Driver async `no_std` pour le capteur AM2302 (DHT22).
//! Compatible avec toutes les cartes via `embedded-hal`.
//!
//! ## Calibration du seuil
//!
//! Le DHT22 encode ses bits par la durée relative du signal haut :
//! ~28 µs → bit `0`, ~70 µs → bit `1`. Cette crate mesure cette durée
//! par **comptage d'itérations de boucle**. Le seuil dépend de la
//! fréquence du MCU — passez la constante correspondant à votre carte :
//!
//! | Carte          | Fréquence | Constante                  |
//! |----------------|-----------|----------------------------|
//! | Raspberry Pi Pico 2 (RP2350) | 150 MHz | [`PICO2_BIT_THRESHOLD`] |
//! | Raspberry Pi Pico (RP2040)   | 125 MHz | [`PICO_BIT_THRESHOLD`]  |
//!
//! Pour toute autre carte, calibrez avec un oscilloscope ou ajustez
//! empiriquement jusqu'à obtenir des lectures cohérentes.
//!
//! ## Exemple — Embassy RP2350
//!
//! ```rust,ignore
//! use embassy_rp::gpio::Flex;
//! use embassy_time::{Duration, Timer, Delay};
//! use embassy_am2302::{am2302_read, PICO2_BIT_THRESHOLD};
//!
//! #[embassy_executor::task]
//! async fn sensor_task(mut pin: Flex<'static>) {
//!     let mut delay = Delay;
//!     loop {
//!         match am2302_read(&mut pin, &mut delay, PICO2_BIT_THRESHOLD).await {
//!             Ok(data) => defmt::info!("{}°C  {}%", data.temp, data.hum),
//!             Err(_)   => {}
//!         }
//!         Timer::after(Duration::from_secs(3)).await;
//!     }
//! }
//! ```

use embedded_hal::digital::{InputPin, OutputPin};
use embedded_hal_async::delay::DelayNs;

/// Seuil calibré pour le **Raspberry Pi Pico 2 (RP2350)** à 150 MHz.
///
/// Passez cette constante à [`am2302_read`] lorsque vous ciblez la Pico 2.
pub const PICO2_BIT_THRESHOLD: u32 = 40;

/// Seuil calibré pour le **Raspberry Pi Pico (RP2040)** à 125 MHz.
///
/// Passez cette constante à [`am2302_read`] lorsque vous ciblez la Pico 1.
pub const PICO_BIT_THRESHOLD: u32 = 33;

/// Données environnementales lues depuis le capteur AM2302.
#[derive(Clone, Copy, Debug)]
pub struct EnvData {
    /// Température en °C. Peut être négative (bit de signe DHT22).
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

/// Lit une mesure depuis le capteur AM2302.
///
/// # Arguments
///
/// * `pin`           — broche GPIO implémentant [`InputPin`] + [`OutputPin`]
/// * `delay`         — implémentation async de [`DelayNs`] fournie par le HAL
/// * `bit_threshold` — seuil de comptage pour distinguer bit `0` et bit `1`.
///   Utilisez [`PICO2_BIT_THRESHOLD`] pour la Pico 2, [`PICO_BIT_THRESHOLD`]
///   pour la Pico 1, ou calibrez pour votre MCU.
///
/// # Retour
///
/// `Ok(EnvData)` si la lecture et le checksum sont valides,
/// `Err(Am2302Error)` sinon.
pub async fn am2302_read<P, E>(
    pin: &mut P,
    delay: &mut impl DelayNs,
    bit_threshold: u32,
) -> Result<EnvData, Am2302Error<E>>
where
    P: InputPin<Error = E> + OutputPin<Error = E>,
{
    // 1. SIGNAL DE START — 20 ms à l'état bas
    pin.set_low().map_err(Am2302Error::Gpio)?;
    delay.delay_ms(20).await;
    pin.set_high().map_err(Am2302Error::Gpio)?;

    // 2. HANDSHAKE
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
    // Signal haut court (< bit_threshold itérations) → bit 0
    // Signal haut long  (> bit_threshold itérations) → bit 1
    let mut data = [0u8; 5];
    for i in 0..40usize {
        timeout = 0;
        while pin.is_low().map_err(Am2302Error::Gpio)? {
            timeout += 1;
            if timeout > 10000 { return Err(Am2302Error::Timeout); }
        }

        let mut high_count = 0u32;
        while pin.is_high().map_err(Am2302Error::Gpio)? {
            high_count += 1;
            if high_count > bit_threshold * 5 { break; }
        }

        if high_count > bit_threshold {
            data[i / 8] |= 1 << (7 - (i % 8));
        }
    }

    // 4. VALIDATION DU CHECKSUM
    let checksum = data[0]
        .wrapping_add(data[1])
        .wrapping_add(data[2])
        .wrapping_add(data[3]);

    if data[4] != checksum {
        return Err(Am2302Error::ChecksumMismatch);
    }

    // 5. DÉCODAGE
    let hum = (((data[0] as u16) << 8) | data[1] as u16) as f32 / 10.0;

    let mut temp = ((((data[2] & 0x7F) as u16) << 8) | data[3] as u16) as f32 / 10.0;
    if data[2] & 0x80 != 0 {
        temp = -temp;
    }

    Ok(EnvData { temp, hum })
}