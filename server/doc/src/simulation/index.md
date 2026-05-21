# Simulation overview

Le sous-système `simulation` rassemble tout ce qui transforme une carte routière en exécution dynamique.

Il se décompose en quatre chapitres:

- [Configuration](config.md) : paramètres globaux d'une simulation et constantes de réglage.
- [Engine details](engine.md) : ordonnancement des véhicules, gestion des feux et progression temporelle.
- [Kinematics helpers](kinematics.md) : formules de vitesse, d'arrivée et de sortie des voies.
- [Vehicle model](vehicle.md) : structure d'un véhicule, trajet, état courant et profil physique.

En pratique, la carte fournit la topologie, le véhicule fournit l'état individuel, la cinématique fournit les bornes physiques, et le moteur orchestre l'ensemble à chaque pas de temps.
