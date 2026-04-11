# embassy-am2302

Driver async `no_std` pour le capteur de température et d'humidité **AM2302 (DHT22)**.
Compatible avec toutes les cartes et tous les exécuteurs async via [`embedded-hal`](https://github.com/rust-embedded/embedded-hal).

> ⚠️ **La version 0.1.0 est obsolète et ne doit pas être utilisée.**
> Elle dépendait d'`embassy-time` et d'`embassy-sync`, ce qui causait des conflits
> de dépendances avec les projets Embassy utilisant une source git.
> Utilise la version **0.2.0** ou supérieure, La version 0.2.1 reste la identique avec un exemple plus réel pour plus de plaisir.

---

## Fonctionnalités

- Lecture de la température et de l'humidité via le protocole 1-Wire du DHT22
- Entièrement asynchrone via `embedded-hal-async`
- **Aucune dépendance Embassy** — compatible avec n'importe quel exécuteur async
- **Aucun timer hardware requis** — mesure des bits par comptage de boucles
- Vérification de la somme de contrôle (checksum) intégrée
- Support des températures négatives
- Compatible `no_std` — aucune allocation dynamique

---

## Matériel supporté

| Capteur | Protocole | Tension |
|---------|-----------|---------|
| AM2302 / DHT22 | 1-Wire | 3.3V – 5V |

Fonctionne avec toute carte dont le HAL implémente `embedded-hal 1.0` :
RP2350, RP2350, STM32, nRF52, ESP32, et plus encore.

---

## Installation

```toml
[dependencies]
embassy-am2302 = "0.2"
```

---

## Utilisation

La crate expose une seule fonction : `am2302_read()`. Elle retourne un `EnvData`
ou une erreur typée. La gestion du signal et de la boucle de lecture
reste dans ton projet, ce qui évite tout conflit de dépendances.

### Exemple complet — Embassy RP2350 avec LCD HD44780

Voici un exemple réel d'intégration dans un projet Embassy RP2350 utilisant
un LCD HD44780 via I2C pour afficher la température et l'humidité.

**`signals.rs`** — déclare le signal dans ton projet :

```rust
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::signal::Signal;
use embassy_am2302::EnvData;

pub static ENV_SIGNAL: Signal<CriticalSectionRawMutex, EnvData> = Signal::new();
```

**`tasks.rs`** — tâche de lecture du capteur :

```rust
use embassy_rp::gpio::Flex; // embassy-rp avec feature rp2350
use embassy_time::{Duration, Timer, Delay};
use embassy_am2302::am2302_read;
use crate::signals::ENV_SIGNAL;

#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut delay = Delay;
    loop {
        if let Ok(data) = am2302_read(&mut pin, &mut delay).await {
            ENV_SIGNAL.signal(data);
        }
        // Timer Embassy pour l'attente entre les lectures —
        // évite de bloquer l'exécuteur pendant 3 secondes
        Timer::after(Duration::from_secs(3)).await;
    }
}
```

**`tasks.rs`** — tâche d'affichage sur LCD (alternance toutes les 3 secondes) :

```rust
use hd44780_i2c_nostd::LcdI2c;
use embassy_rp::i2c::{I2c, Async};
use embassy_rp::peripherals::I2C0;
use embassy_time::{Duration, Timer, Delay};
use core::fmt::Write;
use heapless::String;
use crate::signals::ENV_SIGNAL;

#[embassy_executor::task]
pub async fn lcd_task(mut lcd: LcdI2c<I2c<'static, I2C0, Async>>) {
    let mut delay = Delay;
    let mut show_env = false;

    loop {
        let _ = lcd.clear(&mut delay).await;

        if show_env {
            // --- Affichage AM2302 ---
            if let Some(env) = ENV_SIGNAL.try_take() {
                let _ = lcd.set_cursor(0, 0, &mut delay).await;
                let mut s1: String<16> = String::new();
                let _ = write!(s1, "TEMP: {:.1} C", env.temp);
                let _ = lcd.write_str(&s1, &mut delay).await;

                let _ = lcd.set_cursor(1, 0, &mut delay).await;
                let mut s2: String<16> = String::new();
                let _ = write!(s2, "HUM : {:.1} %", env.hum);
                let _ = lcd.write_str(&s2, &mut delay).await;
            } else {
                // Le capteur ne répond pas encore
                let _ = lcd.set_cursor(0, 0, &mut delay).await;
                let _ = lcd.write_str("AM2302 SENSOR", &mut delay).await;
                let _ = lcd.set_cursor(1, 0, &mut delay).await;
                let _ = lcd.write_str("READING...", &mut delay).await;
            }
        } else {
            // --- Affichage système ---
            let _ = lcd.set_cursor(0, 0, &mut delay).await;
            let _ = lcd.write_str("  JC-OS KERNEL", &mut delay).await;
            let _ = lcd.set_cursor(1, 0, &mut delay).await;
            let _ = lcd.write_str("  SYSTEM READY", &mut delay).await;
        }

        show_env = !show_env;
        Timer::after(Duration::from_secs(3)).await;
    }
}
```

**`main.rs`** — spawn des tâches :

```rust
let pin = Flex::new(p.PIN_2); // broche DATA du AM2302
spawner.spawn(tasks::am2302_task(pin)).unwrap();
spawner.spawn(tasks::lcd_task(lcd)).unwrap();
```

---

### Exemple — Embassy STM32

```rust
use embassy_stm32::gpio::Flex;
use embassy_time::{Duration, Timer, Delay};
use embassy_am2302::am2302_read;

#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut delay = Delay;
    loop {
        if let Ok(data) = am2302_read(&mut pin, &mut delay).await {
            ENV_SIGNAL.signal(data);
        }
        Timer::after(Duration::from_secs(3)).await;
    }
}
```

---

### Gestion fine des erreurs

```rust
use embassy_am2302::Am2302Error;

match am2302_read(&mut pin, &mut delay).await {
    Ok(data)                           => defmt::info!("{}°C  {}%", data.temp, data.hum),
    Err(Am2302Error::ChecksumMismatch) => defmt::warn!("Données corrompues"),
    Err(Am2302Error::Timeout)          => defmt::warn!("Capteur ne répond pas"),
    Err(Am2302Error::Gpio(_))          => defmt::error!("Erreur GPIO"),
}
```

---

## Pourquoi pas de timer hardware ?

Le DHT22 encode ses bits par la **durée relative** du signal haut (~28µs pour un `0`,
~70µs pour un `1`). Cette crate mesure cette durée par **comptage d'itérations de boucle**
plutôt que par un timer matériel, ce qui apporte deux avantages :

- **Zéro dépendance à `embassy-time`** — aucun conflit possible entre versions
- **Portabilité maximale** — fonctionne sur tout MCU sans configuration de timer

Le seuil de détection (40 itérations) est calibré pour un Cortex-M33 à 150 MHz (Raspberry Pi Pico 2).
Sur des MCU significativement plus lents ou plus rapides, ajuste le seuil dans
ta tâche en validant avec un oscilloscope.

---

## Protocole de communication

1. **Signal de start** — la broche est mise à l'état bas pendant 20 ms
2. **Handshake** — le capteur répond avec un signal bas puis haut (~80 µs chacun)
3. **Lecture des 40 bits** — chaque bit est précédé d'un signal bas de ~50 µs :
   - Signal haut court (~28 µs) → bit **0**
   - Signal haut long  (~70 µs) → bit **1**
4. **Validation** — la somme des 4 premiers octets doit correspondre au 5ème (checksum)

---

## Structure des données

```rust
pub struct EnvData {
    pub temp: f32,  // Température en °C (négatif supporté)
    pub hum: f32,   // Humidité relative en %
}
```

---

## Gestion des erreurs

```rust
pub enum Am2302Error<E> {
    Timeout,           // Le capteur ne répond pas
    ChecksumMismatch,  // Données corrompues
    Gpio(E),           // Erreur matérielle HAL
}
```

---

## Migration depuis 0.1.0

| 0.1.0 | 0.2.0 |
|-------|-------|
| `am2302_task(pin)` | `am2302_read(&mut pin, &mut delay)` |
| `am2302_run(pin, delay)` | `am2302_read(&mut pin, &mut delay)` dans ta propre boucle |
| `embassy_am2302::signals::ENV_SIGNAL` | Déclare ton propre `Signal` dans ton projet |
| Dépend d'`embassy-time` et `embassy-sync` | Aucune dépendance Embassy |

---

## Licence

Ce projet est distribué sous licence **GPL-2.0-or-later**.  
Voir le fichier [LICENSE](./LICENSE) pour les détails.