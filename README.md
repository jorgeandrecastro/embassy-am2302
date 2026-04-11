# embassy-am2302

Driver async `no_std` pour le capteur de température et d'humidité **AM2302 (DHT22)**.
Compatible avec toutes les cartes et tous les exécuteurs async via [`embedded-hal`](https://github.com/rust-embedded/embedded-hal).

> ⚠️ **La version 0.1.0 est obsolète et ne doit pas être utilisée.**
> Elle dépendait d'`embassy-time` et d'`embassy-sync`, ce qui causait des conflits
> de dépendances avec les projets Embassy utilisant une source git.
> Utilise la version **0.2.0** ou supérieure.

---

## Fonctionnalités

- Lecture de la température et de l'humidité via le protocole 1-Wire du DHT22
- Entièrement asynchrone via `embedded-hal-async`
- **Aucune dépendance Embassy**  compatible avec n'importe quel exécuteur async
- **Aucun timer hardware requis**  mesure des bits par comptage de boucles
- Vérification de la somme de contrôle (checksum) intégrée
- Support des températures négatives
- Compatible `no_std` —aucune allocation dynamique

---

## Matériel supporté

| Capteur | Protocole | Tension |
|---------|-----------|---------|
| AM2302 / DHT22 | 1-Wire | 3.3V – 5V |

Fonctionne avec toute carte dont le HAL implémente `embedded-hal 1.0` :
RP2040, RP2350, STM32, nRF52, ESP32, et plus encore.

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

### Exemple — Embassy RP2040

```rust
use embassy_rp::gpio::Flex;
use embassy_time::Delay;
use embassy_am2302::{am2302_read, EnvData};

// Dans ton signals.rs :
// pub static ENV_SIGNAL: Signal<CriticalSectionRawMutex, EnvData> = Signal::new();

#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut delay = Delay;
    loop {
        match am2302_read(&mut pin, &mut delay).await {
            Ok(data)  => ENV_SIGNAL.signal(data),
            Err(_)    => {} // lecture ignorée, on réessaie au prochain cycle
        }
        delay.delay_ms(3000).await;
    }
}
```

### Exemple — Embassy STM32

```rust
use embassy_stm32::gpio::Flex;
use embassy_time::Delay;
use embassy_am2302::am2302_read;

#[embassy_executor::task]
pub async fn am2302_task(mut pin: Flex<'static>) {
    let mut delay = Delay;
    loop {
        if let Ok(data) = am2302_read(&mut pin, &mut delay).await {
            ENV_SIGNAL.signal(data);
        }
        delay.delay_ms(3000).await;
    }
}
```

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

- **Zéro dépendance à `embassy-time`** aucun conflit possible entre versions
- **Portabilité maximale** fonctionne sur tout MCU sans configuration de timer

Le seuil de détection (40 itérations) est calibré pour un Cortex-M0+ à 125 MHz.
Sur des MCU significativement plus lents ou plus rapides, ajuste le seuil dans
ta tâche en validant avec un oscilloscope.

---

## Protocole de communication

1. **Signal de start** la broche est mise à l'état bas pendant 20 ms
2. **Handshake**  le capteur répond avec un signal bas puis haut (~80 µs chacun)
3. **Lecture des 40 bits** chaque bit est précédé d'un signal bas de ~50 µs :
   - Signal haut court (~28 µs) → bit **0**
   - Signal haut long  (~70 µs) → bit **1**
4. **Validation** la somme des 4 premiers octets doit correspondre au 5ème (checksum)

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