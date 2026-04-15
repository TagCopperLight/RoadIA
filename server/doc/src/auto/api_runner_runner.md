<!-- Auto documentation: extracted from server/src/api/runner/runner.rs -->

# `api::runner::runner`

Page décrivant la logique d'orchestration des simulations, le contrôleur et
les handlers HTTP/WS exposés par le binaire `server`.

## Rôle principal

- Fournir un `run()` qui démarre le serveur HTTP/WS et expose:
	- `GET /ws` (WebSocket upgrade) — géré par `ws_handler` (défini ailleurs)
	- `POST /api/simulations` — crée une nouvelle instance de simulation
- Gérer les instances de simulation en mémoire (`SimulationInstance`) et
	diffuser périodiquement les `ServerPacket` via un canal `broadcast`.

## Types et champs importants

- `SimulationController` — wrapper léger autour d'un `Arc<AtomicBool>`:
	- `start()`, `stop()`, `is_running()` — contrôle l'exécution de la boucle

- `SimulationInstance` (groupe de runtime pour une simulation):
	- `token: String` — token secret (hex) associé à l'instance (auth WS)
	- `engine: Arc<Mutex<SimulationEngine>>` — état simulation thread-safe
	- `broadcast: broadcast::Sender<ServerPacket>` — canal pour envoyer mises à jour à tous les clients
	- `controller: SimulationController` — contrôle `running`/`stopped`
	- `active_connections: AtomicUsize` — compteur de connexions actives

	- `new(map, vehicles) -> Arc<SimulationInstance>`
		- construit `SimulationEngine` depuis `SimulationConfig` et liste de `Vehicle`
		- crée canal broadcast (100 messages)
		- lance une tâche `tokio::spawn` qui (tant que l'instance vit) :
			- attend que `controller.is_running()` soit `true`
			- appelle `engine.step()` pour progresser d'un pas
			- sérialise véhicules et feux et envoie `ServerPacket::VehicleUpdate`
			- si toutes les voitures sont arrivées ou temps dépassé : calcule `Score`, envoie `ServerPacket::Score` puis `controller.stop()`

- `AppState` — structure partagée contenant `simulations: Arc<RwLock<HashMap<Uuid, Arc<SimulationInstance>>>>`

## Handlers / Endpoints

- `create_simulation_handler(State(state)) -> Json<Value>`
	- Génère un `Uuid`, crée une `SimulationInstance::new_default()` (charge OSM ou panic), insère dans `state.simulations` et retourne `{ "uuid": <uuid>, "token": <hex> }`.
	- Exemple response:

```json
{ "uuid": "550e8400-e29b-41d4-a716-446655440000", "token": "a1b2c3..." }
```

- `run() -> io::Result<()>`:
	- Lit `ALLOWED_ORIGINS` env var (CSV) pour construire un `CorsLayer`.
	- Configure routes `/ws` (get) et `/api/simulations` (post) et démarre Axum sur `0.0.0.0:8080`.

## Détails opérationnels

- Auth / token: le `token` généré par `generate_token()` est une chaîne hex de 32 octets; le WS handler doit valider `uuid`+`token` avant d'accepter et de lier la connexion à l'instance.
- Diffusion: la boucle interne sérialise l'état et envoie via `instance.broadcast.send(...)` — les nouveaux clients doivent s'abonner au `broadcast` pour recevoir `VehicleUpdate` et `Score`.
- Contrôle de fréquence: la boucle calcule `elapsed` et `step_duration` (basé sur `time_step`) et dormira la différence pour garder le pas proche du temps réel.

## Exemples d'utilisation

- Créer une instance via curl (server local):

```bash
curl -X POST http://localhost:8080/api/simulations
```

Réponse: JSON avec `uuid` et `token`. Ensuite ouvrir WS:`/ws?uuid=<uuid>&token=<token>`.

## Notes et recommandations

- `SimulationInstance::new_default()` tente de charger `data/lannion.osm.pbf` et génère des véhicules aléatoires; sur échec il `panic!`.
- `generate_token()` utilise RNG non-cryptographique pour un token d'usage interne — si usage public/production, remplacer par RNG cryptographique.
- `broadcast` taille 100: si les consommateurs sont lents, messages plus anciens seront dropés; le client WS doit lire régulièrement.

---

Souhaites-tu que je transforme ce contenu en page non-`auto` (visuellement plus riche) et j'ajoute un diagramme Mermaid montrant la boucle de la simulation ?

# `src/api/runner/runner.rs`

Overview
- Manages simulation instances and exposes HTTP/WebSocket endpoints to create and control simulations. Responsible for application wiring (CORS, routes) and the long-running background tasks that publish simulation updates.

Types
- `SimulationController`: thread-safe on/off control for a simulation instance (start/stop/is_running).
- `SimulationInstance`: holds the simulation engine, broadcast channel for updates, controller and active connection count; it spawns a background task that advances the engine when running and broadcasts `VehicleUpdate` and `Score` packets.
- `AppState`: shared application state containing the map of `Uuid -> SimulationInstance`.

Key functions
- `generate_token`: internal helper that produces a random token used to authorize WebSocket clients for a simulation instance.
- `create_simulation_handler`: HTTP handler for POST `/api/simulations` which creates a new `SimulationInstance`, stores it in `AppState`, and returns `{ uuid, token }`.
- `run`: initializes shared state, configures CORS and routes, binds a TCP listener and starts the Axum server.

Function parameters and return values (detailed)
- `generate_token() -> String`: no parameters; returns a 32-byte hex string used as an access token for WebSocket connections.
- `create_simulation_handler(State(state): State<Arc<AppState>>) -> Json<Value>`: `state` is the shared `AppState` (contains `simulations` map). The handler creates a `SimulationInstance` (using `new_default()`), inserts it into `state.simulations` with a fresh `Uuid`, and returns JSON `{ "uuid": <uuid>, "token": <token> }`.
- `SimulationInstance::new(map: Map, vehicles: Vec<Vehicle>) -> Arc<Self>`: `map` is a fully-initialized `Map` object, `vehicles` is a precomputed list of `Vehicle` objects. The function returns a reference-counted `Arc` to the created instance; it spawns a background task that locks the engine during steps and publishes `ServerPacket`s via its broadcast channel.
- `SimulationInstance::new_default() -> Arc<Self>`: convenience constructor that attempts to load `data/lannion.osm.pbf` via `create_osm_map` and populates vehicles using `create_random_vehicles(&map, 500)`. Panics if the dataset cannot be loaded.
- `run() -> io::Result<()>`: no parameters. Reads `ALLOWED_ORIGINS` env var (comma-separated origins), builds a `CorsLayer` and Axum `Router` with the routes `/ws` and `/api/simulations`, binds TCP listener `0.0.0.0:8080` and starts the server; returns `Ok(())` on clean shutdown or an `io::Error` on binding failures.

Behavior notes
- `SimulationInstance::new` initializes the engine, populates vehicle paths, and spawns a Tokio task that advances the simulation while `controller.is_running()`, sending periodic `VehicleUpdate` packets and a final `Score` when the run completes.
- `new_default` uses the map generator to load a default OSM dataset (falls back to panic if loading fails).

(This page replaces raw auto-generated signatures with a concise explanation of responsibilities and main APIs.)
