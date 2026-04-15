<!-- Auto documentation: extracted from server/src/map/traffic_light.rs -->

# `map::traffic_light`

Structures légères décrivant la configuration des contrôleurs de feux tricolores.

## Types

- `SignalPhase`
  - `green_link_ids: Vec<u32>` — identifiants des mouvements (links) ouverts pendant la phase verte.
  - `green_duration: f32` — durée verte en secondes.
  - `yellow_duration: f32` — durée jaune en secondes.

- `TrafficLightController`
  - `id: u32` — identifiant unique du contrôleur.
  - `intersection_id: u32` — intersection associée.
  - `phases: Vec<SignalPhase>` — séquence de phases exécutées cycliquement.

- `TrafficLightControllerHandle`
  - `controller_id: u32` — handle utilisé ailleurs pour référencer le contrôleur (ex: runtime / engine).

## Usage et invariants

- Durées exprimées en secondes (float).
- `green_link_ids` référence des `link.id` existants dans le `Map`.
- Le contrôleur est typiquement lié à une intersection; l'engine lit `phases` pour appliquer temporisations.

## Exemple (Rust)

```ignore
let phase = SignalPhase { green_link_ids: vec![101,102], green_duration: 15.0, yellow_duration: 3.0 };
let ctl = TrafficLightController { id: 5, intersection_id: 42, phases: vec![phase] };
```

---

Si tu veux, j'ajoute une page expliquant comment l'engine consomme `TrafficLightController` (synchronisation, événements), et je lie la page au `simulation::engine`.
