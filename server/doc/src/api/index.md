# API Overview

Le backend expose deux surfaces principales:

- un ensemble de routes HTTP pour créer, enregistrer, charger et configurer les simulations;
- un WebSocket `/ws` pour recevoir l'état courant et envoyer les commandes temps réel.

## Pages de référence

- [HTTP handlers](handlers.md) : routes HTTP et structure d'état partagée.
- [Runner / Server entrypoints](runner.md) : contrôleur, instance de simulation et démarrage du serveur.
- [WebSocket protocol](websocket.md) : protocoles JSON échangés avec le client.

## Flux général

Le chemin d'utilisation habituel est le suivant:

1. le client crée une simulation via `POST /api/simulations` ou charge une carte enregistrée;
2. le client ouvre `/ws?uuid=<uuid>&token=<token>`;
3. le serveur envoie l'instantané initial de la carte;
4. le client peut alors démarrer ou arrêter la simulation, éditer la carte ou demander des métriques;
5. le runner publie ensuite les mises à jour de véhicules et, à la fin, le score.
