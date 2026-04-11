// Copyright (C) 2026 Ton Nom
//
// Ce programme est un logiciel libre : vous pouvez le redistribuer et/ou le modifier
// selon les termes de la Licence Publique Générale GNU telle que publiée par la
// Free Software Foundation, soit la version 2 de la licence, soit (à votre convention)
// n'importe quelle version ultérieure.

//! # embassy-am2302
//!
//! Driver async [Embassy](https://embassy.dev/) pour le capteur de température
//! et d'humidité **AM2302 (DHT22)**, conçu pour les systèmes embarqués `no_std`
//! sur microcontrôleurs RP2040 / RP2350.
//!
//! ## Caractéristiques
//!
//! - `#![no_std]`  aucune dépendance à la bibliothèque standard
//! - Entièrement asynchrone via `embassy-executor` et `embassy-time`
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
//! ## Exemple
//!
//! ```rust,ignore
//! use embassy_rp::gpio::Flex;
//! use embassy_am2302::{am2302_task, signals::ENV_SIGNAL};
//!
//! #[embassy_executor::main]
//! async fn main(spawner: Spawner) {
//!     let p = embassy_rp::init(Default::default());
//!     let pin = Flex::new(p.PIN_2);
//!     spawner.spawn(am2302_task(pin)).unwrap();
//! }
//!
//! #[embassy_executor::task]
//! async fn read_env() {
//!     loop {
//!         let data = ENV_SIGNAL.wait().await;
//!         // data.temp → température en °C
//!         // data.hum  → humidité relative en %
//!     }
//! }
//! ```

#![no_std]

pub mod signals;

use embassy_rp::gpio::{Flex, Pull};
use embassy_time::{Duration, Instant, Timer};
use signals::{EnvData, ENV_SIGNAL};

/// Tâche Embassy de lecture continue du capteur AM2302 (DHT22).
///
/// # Comportement
///
/// La tâche tourne indéfiniment. À chaque cycle :
/// 1. Envoie un signal de start (broche à l'état bas pendant 20 ms)
/// 2. Attend le handshake du capteur
/// 3. Lit les 40 bits de données
/// 4. Valide le checksum et publie les données via [`signals::ENV_SIGNAL`]
/// 5. Attend 3 secondes avant la prochaine lecture (contrainte matérielle du DHT22)
///
/// Les lectures dont le checksum est invalide ou dont toutes les données
/// sont nulles sont silencieusement ignorées.
///
/// # Arguments
///
/// * `pin` broche GPIO configurée en mode `Flex<'static>`, connectée
///   au signal DATA du capteur AM2302
///
/// # Exemple
///
/// ```rust,ignore
/// let pin = Flex::new(peripherals.PIN_2);
/// spawner.spawn(am2302_task(pin)).unwrap();
/// ```
#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut data = [0u8; 5];

    loop {
        // 1. SIGNAL DE START
        // La spec DHT22 exige minimum 1ms ; on utilise 20ms pour la robustesse.
        pin.set_as_output();
        pin.set_low();
        Timer::after(Duration::from_millis(20)).await;
        pin.set_high();

        // 2. PASSAGE EN ENTRÉE ET ATTENTE DU HANDSHAKE
        // Séquence : attente bas (80µs) → haut (80µs) → fin handshake
        pin.set_as_input();
        pin.set_pull(Pull::Up);

        let mut timeout = 0;
        while pin.is_high() && timeout < 10000 { timeout += 1; } // attente bas
        timeout = 0;
        while pin.is_low()  && timeout < 10000 { timeout += 1; } // attente haut
        timeout = 0;
        while pin.is_high() && timeout < 10000 { timeout += 1; } // fin handshake

        // 3. LECTURE DES 40 BITS
        // Chaque bit est précédé d'un signal bas de ~50µs.
        // La durée du signal haut détermine la valeur du bit :
        //   < 40µs → bit 0  |  > 40µs → bit 1
        data.fill(0);
        for i in 0..40 {
            while pin.is_low() {}

            let start = Instant::now();
            while pin.is_high() && start.elapsed().as_micros() < 100 {}
            let duration = start.elapsed().as_micros();

            if duration > 40 {
                data[i / 8] |= 1 << (7 - (i % 8));
            }
        }

        // 4. VALIDATION ET PUBLICATION
        // Le checksum est la somme tronquée des 4 premiers octets.
        let checksum = data[0]
            .wrapping_add(data[1])
            .wrapping_add(data[2])
            .wrapping_add(data[3]);

        if data[4] == checksum && (data[0] != 0 || data[2] != 0) {
            let hum = (((data[0] as u16) << 8) | data[1] as u16) as f32 / 10.0;

            // Bit 7 de data[2] = indicateur de signe négatif
            let mut temp = ((((data[2] & 0x7F) as u16) << 8) | data[3] as u16) as f32 / 10.0;
            if (data[2] & 0x80) != 0 {
                temp *= -1.0;
            }

            ENV_SIGNAL.signal(EnvData { temp, hum });
        }

        // Le DHT22 nécessite au minimum 2s entre deux mesures.
        // On attend 3s pour garantir la stabilité.
        Timer::after(Duration::from_secs(3)).await;
    }
}