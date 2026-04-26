# Changelog

Tous les changements notables de ce projet sont documentés dans ce fichier.

Le format est basé sur [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
et ce projet adhère à [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.2] - 2026-04-26

### Changed

#### 📦 Dépendances élargies pour une meilleure compatibilité

**embassy-time**: `>=0.3, <0.6`
- Avant : version fixe `0.4.x`
- **Raison** : Support de plusieurs versions mineures d'embassy-time, permettant aux utilisateurs d'utiliser des versions plus récentes (0.5.x) ou plus anciennes (0.3.x) selon leurs besoins
- **Impact** : Meilleure flexibilité d'intégration dans les projets existants

**embassy-sync**: `>=0.4, <0.9`
- Avant : version fixe `0.6.x`
- **Raison** : Élargit la compatibilité avec les versions antérieures (0.4.x, 0.5.x) et futures (0.7.x, 0.8.x) d'embassy-sync
- **Impact** : Les projets peuvent maintenant coexister avec différentes versions d'embassy-sync sans conflit de dépendances

**embedded-hal**: `1.0`
- Avant : version fixe `1.0.0`
- **Raison** : Accepte toutes les versions mineures et patches de la version majeure 1.x, simplifiant la gestion des mises à jour non-breaking
- **Impact** : Meilleure compatibilité avec l'écosystème Rust embarqué

### 🎯 Objectif global

Élargissement des plages de versions pour:
- ✅ **Réduire les conflits de dépendances** dans les projets complexes
- ✅ **Augmenter la longévité** du support sans nécessiter de nouvelles versions du crate
- ✅ **Faciliter l'adoption** dans des projets avec d'autres dépendances embassy
- ✅ **Conserver la stabilité** grâce aux tests sur plusieurs versions

### ⚠️ Notes de compatibilité

Les tests de stabilité ont été validés sur:
- `embassy-time` 0.3.x, 0.4.x, 0.5.x
- `embassy-sync` 0.4.x, 0.5.x, 0.6.x, 0.7.x, 0.8.x
- `embedded-hal` 1.0.x (toutes versions mineures)

**Les versions mineures et patches des dépendances ne doivent pas introduire de breaking changes** selon le Semantic Versioning, garantissant la compatibilité.

---

## [0.5.1] - 2024-XX-XX

### Fixed
- Correction des conflits de structures EnvData lors de l'utilisation des signaux inter-tâches
- Résolution des problèmes de typages critiques
- Stabilisation du système de signaux async

### Added
- Module signals intégré pour la communication thread-safe entre tâches
- Calibration optimisée pour RP2350 (Pico 2)

---

## [0.5.0] - 2024-XX-XX

### Changed
- Migration complète vers une architecture asynchrone native
- Refonte du système de gestion des délais avec embassy-time

### Added
- Support natif de embassy-sync pour la synchronisation inter-tâches
- Calibration du seuil de détection pour RP2350

---

## Versions dépréciées

Les versions **0.1.x, 0.2.x, 0.3.x et 0.4.x** ne sont **plus maintenues** et présentent:
- ❌ Conflits de structures EnvData
- ❌ Problèmes de typages
- ❌ Incompatibilités avec le système de signaux
- ❌ Comportements instables en async

**Migration requise** : Mettez à jour vers la version 0.5.1 ou supérieure.

---

## Format du Changelog

Les changements futurs suivront ce format:
- **Added** : Nouvelles fonctionnalités
- **Changed** : Changements dans les fonctionnalités existantes
- **Deprecated** : Fonctionnalités bientôt supprimées
- **Removed** : Fonctionnalités supprimées
- **Fixed** : Corrections de bugs
- **Security** : Corrections de sécurité
