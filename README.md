# embassy-am2302 (v0.6.0) 🦅

[![Crates.io](https://img.shields.io/crates/v/embassy-am2302.svg)](https://crates.io/crates/embassy-am2302)
[![Documentation](https://docs.rs/embassy-am2302/badge.svg)](https://docs.rs/embassy-am2302)
[![License: GPL-2.0-or-later](https://img.shields.io/badge/license-GPL--2.0--or--later-blue.svg)](https://opensource.org/licenses/GPL-2.0-or-later)

Driver async no_std pour le capteur de température et d'humidité AM2302 (DHT22).

Conçu spécifiquement pour l'écosystème Embassy (embassy-time, embassy-sync).

## ⚠️ VERSIONS DÉPRÉCIÉES

**Les versions 0.1.x, 0.2.x, 0.3.x et 0.4.x sont DÉPRÉCIÉES et ne doivent plus être utilisées.**

Ces versions anciennes souffrent de :
- Conflits de structures EnvData inter-tâches
- Problèmes de typages critiques
- Incompatibilités avec le système de signaux
- Comportements instables en async

---

# 🔄 Fixed

**Version actuelle conseillée : v0.6.0**

Dépendances désormais fixées pour une meilleure stabilité
Amélioration notable par rapport à la version v0.5.2, notamment sur l’écosystème Embassy (embassy-time, embassy-sync)
Suppression des plages de versions larges afin d’éviter le dependency hell

---


## 📝 Changelog

Pour voir l'historique complet des changements, des améliorations et des corrections apportées au projet, consultez le fichier [CHANGELOG.md](CHANGELOG.md).

**Version actuelle** : v0.6.0  

Dépendances désormais fixées pour une meilleure stabilité
Amélioration notable par rapport à la version v0.5.2, notamment sur l’écosystème Embassy (embassy-time, embassy-sync)
Suppression des plages de versions larges afin d’éviter le dependency hell

----


## Fonctionnalités

- **Async Natif** : Entièrement non-bloquant via embassy-time.
- **Calibration RP2350** : Testé et validé sur Pico 2 avec des seuils adaptés à la vitesse du processeur.
- **Découplage Inter-tâches** : Module signals intégré pour une communication thread-safe entre vos tâches.
- **Zéro Allocation** : Idéal pour les systèmes bare-metal .

----

## Installation

```toml
[dependencies]
embassy-am2302 = "0.6.0"
embassy-time  = "0.5" 
embassy-sync  = "0.8" 
embedded-hal  = "1.0"  
```
----

## Constantes de seuil (Calibration)

Le DHT22 mesure la durée des impulsions. La vitesse du processeur influence le comptage. Voici les valeurs validées :

| Constante | Carte | Fréquence | Seuil Recommandé |
|-----------|-------|-----------|-----------------|
| `PICO2_BIT_THRESHOLD` | Raspberry Pi Pico 2 | 150 MHz | 120 (Validé) |
| `PICO_BIT_THRESHOLD` | Raspberry Pi Pico | 125 MHz | 40 |

**Note** : Si vous utilisez des câbles longs, privilégiez un seuil plus élevé.

---

## Exemple Complet : Intégration  (LCD + Capteur)

**Voici comment orchestrer le capteur et un écran LCD HD44780 en utilisant le multitâche Embassy.**

```rust
#![no_std]
#![no_main]

use embassy_executor::Spawner;
use embassy_rp::gpio::{Flex, Pull};
use embassy_rp::i2c::{Config as I2cConfig, I2c};
use embassy_time::{Delay, Duration, Timer};
use hd44780_i2c_nostd::LcdI2c;
use core::fmt::Write;
use heapless::String;

// Utilisation du driver et du signal global
use embassy_am2302::{am2302_read, EnvData};
use embassy_am2302::signals::ENV_SIGNAL;

#[embassy_executor::task]
async fn sensor_task(mut pin: Flex<'static>) {
    loop {
        // Seuil 120 optimisé pour la vitesse de la Pico 2
        match am2302_read(&mut pin, 120).await { 
            Ok(data) => ENV_SIGNAL.signal(data),
            Err(_)   => ENV_SIGNAL.signal(EnvData { temp: 999.0, hum: 0.0 }),
        }
        Timer::after(Duration::from_millis(2500)).await;
    }
}

#[embassy_executor::task]
async fn display_task(mut lcd: LcdI2c<I2c<'static, embassy_rp::peripherals::I2C0, embassy_rp::i2c::Async>>) {
    let mut delay = Delay;
    lcd.init(&mut delay).await.ok();
    lcd.set_backlight(true).ok();

    loop {
        let data = ENV_SIGNAL.wait().await; // Attend la mesure de sensor_task
        let _ = lcd.clear(&mut delay).await;
        
        let mut s: String<16> = String::new();
        if data.temp > 500.0 {
            write!(lcd, "SENSOR ERROR").ok();
        } else {
            write!(s, "T: {:.1} C", data.temp).ok();
            lcd.set_cursor(0, 0, &mut delay).await.ok();
            lcd.write_str(s.as_str(), &mut delay).await.ok();
        }
    }
}
```
---

## Schéma de Câblage (exemple RP2350a)

Pour éviter les erreurs 999.0 (Sensor Error), respectez scrupuleusement ce montage :

- **VCC** : Reliez au VBUS (5V) pour plus de stabilité.
- **DATA** : Pin GP22 (Pin 29 physique).
- **GND** : Masse commune.
- **PULL-UP** : Ajoutez une résistance physique de 4.7kΩ entre DATA et 3.3V.

---

## Pourquoi cette architecture ?

L'utilisation du ENV_SIGNAL intégré permet un découplage total :

- Votre tâche de lecture gère le timing critique du capteur.
- Votre tâche d'affichage (ou de log) réagit instantanément dès qu'une donnée est disponible.
- Aucune variable globale risquée (`static mut`), tout passe par un Signal sécurisé par section critique.

----

## Licence

Ce projet est distribué sous licence GPL-2.0-or-later.

Voir le fichier LICENSE pour les détails.

----

## Copyright

Copyright (C) 2026 Jorge Andre Castro