# RoadIA

<<<<<<< HEAD
Interactive road network editor and real-time traffic simulator. Design road layouts, set vehicle waypoints, and watch traffic flow.
=======
RoadIA est un éditeur de réseau routier et un simulateur de trafic en temps réel. Ce projet permet de créer des routes, de définir des points de passage pour les véhicules et d'observer la circulation.
>>>>>>> origin/dev

## Tech stack

| | Frontend | Backend |
|---|---|---|
| **Language** | TypeScript | Rust |
| **Framework** | Next.js 15 | Axum |
| **Rendering** | Pixi.js, Leaflet | — |
| **Communication** | WebSocket | Tokio + WebSocket |

---

<<<<<<< HEAD
## Local development

### Requirements

- **Frontend**: Node.js 25+, npm
- **Backend**: Rust stable (1.88+), Cargo

### Frontend
=======
## Installation et lancement local

### 1. Cloner le projet avec Git

```bash
git clone https://github.com/TagCopperLight/RoadIA
cd RoadIA
```

### 2. Prérequis

- **Frontend**: Node.js 25+, npm
- **Backend**: Rust stable (1.88+), Cargo
- **Optionnel**: Docker et Docker Compose si vous souhaitez lancer l'application via conteneurs

### 3. Installer et lancer le frontend

Le frontend Next.js se trouve dans le dossier client/.
>>>>>>> origin/dev

```bash
cd client
npm install
<<<<<<< HEAD
npm run dev       # dev server → http://localhost:3000
```

Other scripts:

```bash
npm run build     # production build
npm start         # serve production build
npm run lint      # run ESLint
```

### Backend

```bash
cd server
cargo run                    # run in debug mode
cargo build --release        # release build
cargo test --all-features    # run tests
=======
npm run dev
```

Le serveur de développement sera disponible sur http://localhost:3000.

Commandes utiles:

```bash
npm run build     # génération de la version de production
npm start         # lancer la version de production
```

### 4. Installer et lancer le backend

Le backend Rust se trouve dans le dossier server/.

```bash
cd ../server
cargo run
```

Commandes utiles:

```bash
cargo build --release   # compilation optimisée
cargo test --all-features   # exécution des tests
>>>>>>> origin/dev
```

---

## Docker

<<<<<<< HEAD
### Requirements

- Docker
- Docker Compose

### Run
=======
### Prérequis

- Docker 20.10+
- Docker Compose v2 (plugin, `docker compose`)

### Lancement
>>>>>>> origin/dev

```bash
docker compose up --build
```

Services:
<<<<<<< HEAD
- **client** — Next.js app
- **server** — Rust API + simulation engine
=======
- **client** — application Next.js
- **server** — API Rust et moteur de simulation
>>>>>>> origin/dev

---

## Environment variables

| Variable | Service | Default | Description |
|---|---|---|---|
| `NEXT_PUBLIC_API_URL` | client | `http://localhost:8080` | URL of the backend API |
| `ALLOWED_ORIGINS` | server | `http://localhost:3000` | CORS allowed origins |
