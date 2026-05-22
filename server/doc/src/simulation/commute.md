# Plans de trajet pendulaire (Commute)

Le système de trajet pendulaire (`CommutePlan`) gère les cycles de déplacement aller-retour des travailleurs et des résidents, permettant de simuler des journées complètes de trafic réaliste.

## Structure du plan

Un `CommutePlan` lie deux véhicules (ou un véhicule effectuant deux trajets) pour représenter un cycle complet :

| Champ | Description |
|---|---|
| `id` | Identifiant unique du plan. |
| `outbound_vehicle_id` | ID du véhicule pour le trajet aller (ex: Domicile -> Travail). |
| `return_vehicle_id` | ID du véhicule pour le trajet retour (ex: Travail -> Domicile). |
| `outbound_departure_time_s` | Heure de départ prévue pour l'aller. |
| `return_waiting_time_s` | Temps d'attente à destination avant le retour. |
| `state` | État actuel du cycle (`OutboundPending`, `Running`, `Completed`, etc.). |

## Distribution temporelle

Pour générer un trafic réaliste, les heures de départ et les temps d'attente ne sont pas uniformes mais suivent des **lois Beta**, simulant les pics de trafic du matin et du soir.

### Paramètres de distribution
- **Fenêtre de temps** : 12 heures (43 200 s) pour chaque phase.
- **Départ (Aller)** : Paramètres α=6.33, β=3.67. Cette distribution place le pic de départ vers le milieu/fin de la matinée.
- **Attente (Travail)** : Paramètres α=7.25, β=2.75. Simule une journée de travail standard de plusieurs heures.

## Cycle de vie

1. **OutboundPending** : Le plan est créé et attend l'heure `outbound_departure_time_s`.
2. **OutboundRunning** : Le véhicule aller est injecté dans la simulation.
3. **WaitingForReturnDeparture** : Une fois l'aller arrivé, le moteur calcule l'heure de retour : `heure_arrivée + return_waiting_time_s`.
4. **ReturnRunning** : Le véhicule de retour est injecté.
5. **Completed** : Le cycle est terminé après l'arrivée du retour.

## Intégration dans le moteur

Le `SimulationEngine` gère une liste de `commute_plans`. À chaque pas, il vérifie quels plans doivent passer à l'étape suivante via la méthode `handle_commutes()`.
