# embassy-am2302

Driver async `no_std` pour le capteur de température et d'humidité **AM2302 (DHT22)**.
Compatible avec toutes les cartes via [`embedded-hal`](https://github.com/rust-embedded/embedded-hal).

> ⚠️ **La version 0.1.0 est obsolète.** Elle dépendait d'`embassy-time` et `embassy-sync`,
> ce qui causait des conflits avec les projets Embassy utilisant une source git.
> La version 0.2.x corrigeait cela mais utilisait un seuil fixe non documenté.
> La **0.3.0** expose le seuil en paramètre avec des constantes prêtes à l'emploi.

---

## Fonctionnalités

- Entièrement asynchrone via `embedded-hal-async`
- **Aucune dépendance Embassy** — zéro conflit de versions
- **Aucun timer hardware requis** — mesure des bits par comptage de boucles
- Seuil de détection **passé en paramètre** — portable sur tout MCU
- Constantes précalibrées pour la Pico 2 et la Pico 1
- Vérification du checksum intégrée
- Support des températures négatives
- Compatible `no_std`, aucune allocation dynamique

---

## Installation

```toml
[dependencies]
embassy-am2302 = "0.3"
```

---

## Utilisation

```rust
use embassy_am2302::{am2302_read, PICO2_BIT_THRESHOLD};

match am2302_read(&mut pin, &mut delay, PICO2_BIT_THRESHOLD).await {
    Ok(data)                           => defmt::info!("{}°C  {}%", data.temp, data.hum),
    Err(Am2302Error::ChecksumMismatch) => defmt::warn!("Données corrompues"),
    Err(Am2302Error::Timeout)          => defmt::warn!("Capteur ne répond pas"),
    Err(Am2302Error::Gpio(_))          => defmt::error!("Erreur GPIO"),
}
```

---

## Constantes de seuil

| Constante | Carte | Fréquence |
|-----------|-------|-----------|
| `PICO2_BIT_THRESHOLD` | Raspberry Pi Pico 2 (RP2350) | 150 MHz |
| `PICO_BIT_THRESHOLD`  | Raspberry Pi Pico (RP2040)   | 125 MHz |

Pour tout autre MCU, calibrez empiriquement ou avec un oscilloscope.
La valeur correcte se situe entre ~28 µs et ~70 µs selon votre fréquence :
`threshold = fréquence_mhz * 28 / 1000` comme point de départ.

---

## Exemple complet — Embassy RP2350 avec LCD HD44780

**`signals.rs`** :

```rust
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_am2302::EnvData;

pub static ENV_SIGNAL: Signal<CriticalSectionRawMutex, EnvData> = Signal::new();
```

**`tasks.rs`** — lecture capteur :

```rust
use embassy_rp::gpio::Flex;
use embassy_time::{Duration, Timer, Delay};
use embassy_am2302::{am2302_read, PICO2_BIT_THRESHOLD};
use crate::signals::ENV_SIGNAL;

#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut delay = Delay;
    loop {
        if let Ok(data) = am2302_read(&mut pin, &mut delay, PICO2_BIT_THRESHOLD).await {
            ENV_SIGNAL.signal(data);
        }
        Timer::after(Duration::from_secs(3)).await;
    }
}
```

**`tasks.rs`** — affichage LCD :

```rust
use hd44780_i2c_nostd::LcdI2c;
use embassy_rp::i2c::{I2c, Async};
use embassy_rp::peripherals::I2C0;
use embassy_time::{Delay, Timer, Duration};
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
            if let Some(env) = ENV_SIGNAL.try_take() {
                let _ = lcd.set_cursor(0, 0, &mut delay).await;
                let mut s: String<16> = String::new();
                let _ = write!(s, "TEMP: {:.1} C", env.temp);
                let _ = lcd.write_str(&s, &mut delay).await;

                let _ = lcd.set_cursor(1, 0, &mut delay).await;
                s.clear();
                let _ = write!(s, "HUM : {:.1} %", env.hum);
                let _ = lcd.write_str(&s, &mut delay).await;
            } else {
                let _ = lcd.set_cursor(0, 0, &mut delay).await;
                let _ = lcd.write_str("AM2302 SENSOR", &mut delay).await;
                let _ = lcd.set_cursor(1, 0, &mut delay).await;
                let _ = lcd.write_str("READING...", &mut delay).await;
            }
        } else {
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

**`main.rs`** :

```rust
let pin = Flex::new(p.PIN_2);
spawner.spawn(tasks::am2302_task(pin)).unwrap();
spawner.spawn(tasks::lcd_task(lcd)).unwrap();
```

---

## Pourquoi un seuil en paramètre ?

Le DHT22 encode ses bits par la **durée relative** du signal haut (~28 µs pour un `0`,
~70 µs pour un `1`). Cette crate mesure cette durée par comptage d'itérations de boucle,
ce qui évite toute dépendance à `embassy-time` et tout conflit de versions.

En contrepartie, le nombre d'itérations correspondant à 28–70 µs varie selon la fréquence
du MCU. Exposer ce seuil en paramètre plutôt que de le coder en dur rend la crate portable
sans introduire la moindre dépendance Embassy.

---

## Migration depuis 0.2.x

| 0.2.x | 0.3.0 |
|-------|-------|
| `am2302_read(&mut pin, &mut delay)` | `am2302_read(&mut pin, &mut delay, PICO2_BIT_THRESHOLD)` |
| Seuil fixe interne (40) | Seuil explicite passé en argument |
| Non documenté pour autres MCU | Constantes fournies + formule de calibration |

---

## Licence

Ce projet est distribué sous licence **GPL-2.0-or-later**.
Voir le fichier [LICENSE](./LICENSE) pour les détails.