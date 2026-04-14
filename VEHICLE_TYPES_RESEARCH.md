# Recherche : Types de Véhicules - Données Réelles 2025-2026

## 📊 SOURCES OFFICIELLES

### 1. SDES (Service des Données et Études Statistiques)
- **Lien officiel :** https://www.statistiques.developpement-durable.gouv.fr/
- **Dernière mise à jour :** Mars 2026
- **Autorité :** Ministère de la Transition écologique - Données certificatoires

---

## 🚗 RÉPARTITION DES MOTORISATIONS EN FRANCE (2025-2026)

### Immatriculations neuves - Mars 2026 (Source SDES)
| Motorisation | Part % | Évolution |
|---|---|---|
| **Essence hybride non-rechargeable** | 45,0% | -1,4 pts |
| **Électrique** | 28,1% | +1,7 pts 🔼 (record) |
| **Essence thermique** | 14,8% | -0,8 pts |
| **Hybride rechargeable** | 4,6% | -0,8 pts |
| **Diesel + diesel hybride** | 3,4% | stable |
| **Diesel thermique** | 2,6% | stable |

**Source :** SDES - "Motorisations des véhicules légers neufs - Émissions de CO2 et bonus écologique - Mars 2026"
https://www.statistiques.developpement-durable.gouv.fr/motorisations-des-vehicules-legers-neufs-emissions-de-co2-et-bonus-ecologique-mars-2026

### Immatriculations neuves - Année 2025 (Source SDES)
| Motorisation | Part % | Volume |
|---|---|---|
| **Essence hybride non-rechargeable** | 42,5% | ~708k véhicules |
| **Électrique** | 19,9% | ~331k véhicules |
| **Essence thermique** | 26,7% | (inclus cidessus) |
| **Diesel + diesel hybride** | ~10% | ~166k véhicules |
| **Hybride rechargeable** | 6,6% | ~110k véhicules |

**Total 2025 :** 1,665 million de voitures neuves immatriculées (-5,2% vs 2024, -26,3% vs 2019)

**Source :** SDES - "Immatriculations de voitures en 2025 : le marché du neuf baisse, celui de l'occasion résiste"
https://www.statistiques.developpement-durable.gouv.fr/immatriculations-de-voitures-en-2025-le-marche-du-neuf-baisse-celui-de-loccasion-resiste

---

## 📏 DIMENSIONS MOYENNES DES VOITURES PAR SEGMENT

### Données : Parc automobile français moyen

| Segment | Longueur (m) | Largeur (m) | Masse (kg) | Exemples |
|---|---|---|---|---|
| **Micro** (A) | 3,3-3,7 | 1,6 | 800-1000 | Citroën C1, Peugeot 108 |
| **Citadine** (B) | 3,9-4,3 | 1,7 | 1000-1200 | Renault Clio, Peugeot 208 |
| **Compacte** (C) | 4,3-4,6 | 1,8 | 1200-1400 | Renault Megane, VW Golf |
| **Berline** (D) | 4,7-4,9 | 1,8 | 1400-1600 | BMW 320, Mercedes C-Class |
| **SUV compact** | 4,3-4,6 | 1,8 | 1300-1600 | Renault Captur, Peugeot 2008 |
| **SUV mid-size** | 4,6-4,9 | 1,9 | 1500-1800 | Renault Espace, VW Tiguan |
| **4x4/SUV large** | 4,8-5,2 | 1,95 | 1700-2100 | BMW X5, Lincoln Navigator |

**Sources :**
- ADAC (Automobile Club Allemand) - Données constructeur 2024-2025
- EPA (US Environmental Protection Agency) - Vehicle classes
- CITEPA - Inventaire des véhicules routiers

---

## 💨 ÉMISSIONS DE CO2 PAR TYPE DE VÉHICULE
### Données cycle WLTP (réel roulage) 2024-2026

| Type de motorisation | CO2 (g/km) WLTP | Consommation | Notes |
|---|---|---|---|
| **Essence thermique** | 140-180 | 5,5-7,0 L/100km | Segment C moyen |
| **Diesel thermique** | 110-150 | 4,5-5,8 L/100km | Plus efficace |
| **Essence hybride** | 90-120 | 3,5-4,7 L/100km | Non-rechargeable |
| **Hybride rechargeable** | 40-80 | 1,5-3,0 L/100km* | *Avec recharge optimale |
| **Électrique** | 0-60 | 0 L/100km | Selon mix électricité |

**Moyenne parc neuf 2025 :** ~115 g CO2/km (baisse vs 2024)

**Sources :**
- ICCT (International Council on Clean Transportation) - WLTP standards
- CITEPA - Inventaires d'émissions 2025
- EPA - Fuel Economy Guide

---

## 🎯 PROPOSITION D'IMPLÉMENTATION : 4 TYPES RÉALISTES

### Basé sur la distribution réelle France 2025-2026

```
TYPE = (Motorisation, Segment)
```

| Type | Motorisation | Segment | Poids % | Taille client | Exemples |
|---|---|---|---|---|---|
| **Economy** | Essence hybride | Citadine (B) | 20% | 5m x 1,7m | Clio, 208 |
| **Standard** | Essence hybride | Compacte (C) | 35% | 5m x 1,8m | Megane, Golf |
| **Premium** | Électrique | Berline (D) | 25% | 5,3m x 1,8m | Tesla Model 3, ID.5 |
| **Eco** | Électrique | Micro (A) | 20% | 4m x 1,6m | Renault Zoe, ID.3 |

### Alternative : 5 TYPES (Plus détaillé)

| Type | Motorisation | Segment | % parc |
|---|---|---|---|
| **Micro Eco** | Électrique | A | 10% |
| **City** | Essence hybride | B | 20% |
| **Urban** | Essence hybride | C | 30% |
| **Electric** | Électrique | C-D | 20% |
| **Premium** | Hybride rechargeable | D | 10% |
| **Eco Diesel** | Diesel | C | 10% |

**À noter :** Le diesel a drastiquement baissé → d'ici 2026, représente <5% du neuf (mais 44% de l'occasion)

---

## 📋 VÉHICULES UTILITAIRES (Hors champ initial)

| Type | CO2 (g/km) | Motorisation | % immat. |
|---|---|---|---|
| Fourgonnette | 180-220 | Diesel | 65% |
| Petit utilitaire | 150-200 | Diesel | 70% |
| Camionnette | 200-240 | Diesel | 90% |

*Non prioritaire pour RoadIA (focus voitures particulières)*

---

## 🔬 DONNÉES DE RÉFÉRENCE POUR LE CODE

### Taille réelle des voitures
**Actuellement dans RoadIA :** 8px × 5px (très petit)
- **Échelle proposée :** 1 pixel = 0,5 mètres réels
  - Micro (4m) → 8px largeur ✓
  - Standard (5m) → 10px largeur
  - Large (5,5m) → 11px largeur
  - Truck (6m) → 12px largeur

---

##  RECOMMANDATIONS

### ✅ À UTILISER

1. **Données SDES** pour distribution motorisations
2. **Données CITEPA** pour émissions CO2
3. **Données constructeurs officielles** pour dimensions

### ⚠️ À ÉVITER

- Web scraping non-officiel
- Données de sites marchands (prix variables)
- Données anciennes >2023

### 📌 NEXT STEPS

1. Créer 4 types de véhicules basés sur dist réelle
2. Ajouter champs : motorisation, masse, dimensions
3. Implémenter calcul CO2 réaliste par type
4. Tester randomisation selon distribution France 2025

---

**Dernière mise à jour :** Avril 2026
**Sources vérifiées :** SDES, CITEPA, statut officiel
**Utilisable pour :** Simulation réaliste trafic urbain France
