# embassy-am2302 (v0.5.0) 🦅

Driver async no_std pour le capteur de température et d'humidité AM2302 (DHT22).

Conçu spécifiquement pour l'écosystème Embassy (embassy-time, embassy-sync) et compatible avec toutes les cartes grâce à embedded-hal.

> ⚠️ **IMPORTANT** : Les versions antérieures (0.2.x à 0.4.x) sont obsolètes suite à des problèmes de typage inter-tâches. La v0.5.0 est la version stable recommandée qui corrige les conflits de structures EnvData lors de l'utilisation des signaux.

## Fonctionnalités

- **Async Natif** : Entièrement asynchrone via embassy-time.
- **Universel** : Compatible avec embassy-rp, embassy-stm32, embassy-nrf, etc.
- **Ultra-Précis** : Mesure des bits par comptage de boucles (évite le jitter des interruptions).
- **Découplage Inter-tâches** : Inclut un module signals pour envoyer les données entre vos tâches sans effort.
- **Zéro Allocation** : Compatible no_std, idéal pour les kernels comme JC-OS.

## Installation

```toml
[dependencies]
embassy-am2302 = "0.5.0"
embassy-time   = { version = "0.4.0" }
embassy-sync   = { version = "0.6.0" }
embedded-hal   = { version = "1.0" }
```

## Utilisation Rapide

```rust
use embassy_am2302::{am2302_read, PICO2_BIT_THRESHOLD};

// Lecture directe (besoin d'une pin implémentant InputPin + OutputPin, ex: Flex)
match am2302_read(&mut pin, PICO2_BIT_THRESHOLD).await {
    Ok(data) => defmt::info!("{:.1}°C  {:.1}%", data.temp, data.hum),
    Err(e)   => defmt::error!("Erreur lecture : {:?}", e),
}
```

## Constantes de seuil (Calibration)

Le DHT22 est sensible à la fréquence de votre CPU. Utilisez ces constantes ou calculez la vôtre :

| Constante | Carte | Fréquence |
|-----------|-------|-----------|
| `PICO2_BIT_THRESHOLD` | Raspberry Pi Pico 2 (RP2350) | 150 MHz |
| `PICO_BIT_THRESHOLD` | Raspberry Pi Pico (RP2040) | 125 MHz |

## Architecture Multi-tâches (Exemple Recommandé)

Pour un projet propre (comme dans JC-OS), utilisez le signal intégré pour séparer la lecture de l'affichage.

### 1. Le Signal (signals.rs)

Le signal utilise une section critique pour être accessible partout.

```rust
use embassy_am2302::signals::ENV_SIGNAL;
```

### 2. Tâche de lecture

```rust
#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    loop {
        // Le capteur est vicieux, on ignore les erreurs isolées
        if let Ok(data) = am2302_read(&mut pin, PICO2_BIT_THRESHOLD).await {
            ENV_SIGNAL.signal(data); // Envoi au reste du système
        }
        Timer::after(Duration::from_secs(2)).await; // Respecter le cycle du DHT22
    }
}
```

### 3. Tâche d'affichage (LCD HD44780)

```rust
#[embassy_executor::task]
pub async fn lcd_task(mut lcd: LcdI2c<I2c<'static, I2C0, Async>>) {
    loop {
        let env = ENV_SIGNAL.wait().await; // Attend une nouvelle mesure
        
        let mut s: String<16> = String::new();
        write!(s, "{:.1}C  {:.1}%", env.temp, env.hum).unwrap();
        
        lcd.clear(&mut Delay).await.ok();
        lcd.write_str(s.as_str(), &mut Delay).await.ok();
    }
}
```

## Pourquoi cette version ?

- Le protocole du AM2302 est basé sur des durées de microsecondes (28µs pour un 0, 70µs pour un 1).
- **embedded-hal** : Utilisé pour l'abstraction des pins, permettant d'utiliser ce driver sur n'importe quel microcontrôleur.
- **embassy-time** : Assure que le signal de "Start" (20ms) est respecté sans bloquer les autres tâches asynchrones.
- **signals** : Correction majeure de la v0.5.0 assurant que EnvData est un type unique et cohérent dans toute l'application.

## Copyright

Copyright (C) 2026 Jorge Andre Castro



## Licence

Ce projet est distribué sous licence GPL-2.0-or-later.

Voir le fichier LICENSE pour les détails.