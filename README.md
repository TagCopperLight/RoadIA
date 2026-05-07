# RoadIA

Interactive road network editor and real-time traffic simulator. Design road layouts, set vehicle waypoints, and watch traffic flow.

## Tech stack

| | Frontend | Backend |
|---|---|---|
| **Language** | TypeScript | Rust |
| **Framework** | Next.js 15 | Axum |
| **Rendering** | Pixi.js, Leaflet | — |
| **Communication** | WebSocket | Tokio + WebSocket |

---

## Local development

### Requirements

- **Frontend**: Node.js 25+, npm
- **Backend**: Rust stable (1.88+), Cargo

### Frontend

```bash
cd client
npm install
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
```

---

## Docker

### Requirements

- Docker 20.10+
- Docker Compose v2 (plugin, `docker compose`)

### Run

```bash
docker compose up --build
```

Services:
- **client** — Next.js app
- **server** — Rust API + simulation engine

---

## Environment variables

| Variable | Service | Default | Description |
|---|---|---|---|
| `NEXT_PUBLIC_API_URL` | client | `http://localhost:8080` | URL of the backend API |
| `ALLOWED_ORIGINS` | server | `http://localhost:3000` | CORS allowed origins |
