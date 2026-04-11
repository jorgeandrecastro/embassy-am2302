# embassy-am2302

Driver async [Embassy](https://embassy.dev/) pour le capteur de température et d'humidité **AM2302 (DHT22)**, conçu pour les systèmes embarqués `no_std` sur microcontrôleurs RP2040.

---

## Fonctionnalités

- Lecture de la température et de l'humidité via le protocole 1-Wire du DHT22
- Entièrement asynchrone grâce à `embassy-executor` et `embassy-time`
- Vérification de la somme de contrôle (checksum) intégrée
- Support des températures négatives
- Compatible `no_std` — aucune allocation dynamique

---

## Matériel supporté

| Capteur | Protocole | Tension |
|---------|-----------|---------|
| AM2302 / DHT22 | 1-Wire | 3.3V – 5V |

---

## Installation

Ajoute la dépendance dans ton `Cargo.toml` :

```toml
[dependencies]
embassy-am2302 = "0.1"
```

---

## Utilisation

```rust
use embassy_rp::gpio::Flex;
use embassy_am2302::am2302_task;

#[embassy_executor::main]
async fn main(spawner: Spawner) {
    let p = embassy_rp::init(Default::default());
    let pin = Flex::new(p.PIN_2);

    spawner.spawn(am2302_task(pin)).unwrap();
}
```

Le capteur envoie ses données toutes les **3 secondes** via un `Signal` Embassy (`ENV_SIGNAL`).

Pour récupérer les données dans une autre tâche :

```rust
use embassy_am2302::signals::{ENV_SIGNAL, EnvData};

#[embassy_executor::task]
async fn read_env() {
    loop {
        let data: EnvData = ENV_SIGNAL.wait().await;
        defmt::info!("Temp: {}°C  Humidité: {}%", data.temp, data.hum);
    }
}
```

---

## Protocole de communication

Le driver implémente le protocole officiel du DHT22 :

1. **Signal de start** — la broche est mise à l'état bas pendant 20 ms
2. **Handshake** — le capteur répond avec un signal bas puis haut
3. **Lecture des 40 bits** — chaque bit est encodé par la durée du signal haut :
   - `< 40 µs` → bit **0**
   - `> 40 µs` → bit **1**
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

## Limitations connues

- La broche doit être déclarée comme `Flex<'static>`
- Le capteur nécessite un délai minimum de **3 secondes** entre deux lectures
- Pas de gestion d'erreur explicite (les lectures invalides sont silencieusement ignorées)

---

## Licence

Ce projet est distribué sous licence **GPL-2.0-or-later**.  
Voir le fichier [LICENSE](./LICENSE) pour les détails.