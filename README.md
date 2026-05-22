# RoadIA

RoadIA est un éditeur de réseau routier et un simulateur de trafic en temps réel. Ce projet permet de créer des routes, de définir des points de passage pour les véhicules et d'observer la circulation.

## Tech stack

| | Frontend | Backend |
|---|---|---|
| **Language** | TypeScript | Rust |
| **Framework** | Next.js 15 | Axum |
| **Rendering** | Pixi.js, Leaflet | — |
| **Communication** | WebSocket | Tokio + WebSocket |

---

## Installation et lancement avec Docker

### 1. Cloner le projet avec Git

```bash
git clone https://github.com/TagCopperLight/RoadIA
cd RoadIA
```

ou cliquer sur `Code` puis `Download ZIP`.

### 2. Prérequis

- Docker 20.10+
- Docker Compose v2 (plugin, `docker compose`)

### 3. Lancement

```bash
docker compose up --build
```

Services:
- **client** — application Next.js
- **server** — API Rust et moteur de simulation

Le serveur de développement sera disponible sur http://localhost:3000.

---

## Installation et lancement local

### 1. Cloner le projet avec Git

```bash
git clone https://github.com/TagCopperLight/RoadIA
cd RoadIA
```

ou cliquer sur `Code` puis `Download ZIP`.

### 2. Prérequis

- **Frontend**: Node.js 25+, npm
- **Backend**: Rust stable (1.88+), Cargo

### 3. Installer et lancer le frontend

Le frontend Next.js se trouve dans le dossier client/.

```bash
cd client
npm install
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
```

Il est aussi possible de télécharger un binaire précompilé sur la page [releases](https://github.com/TagCopperLight/RoadIA/releases).

---

## Environment variables

| Variable | Service | Default | Description |
|---|---|---|---|
| `NEXT_PUBLIC_API_URL` | client | `http://localhost:8080` | URL of the backend API |
| `ALLOWED_ORIGINS` | server | `http://localhost:3000` | CORS allowed origins |
