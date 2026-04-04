# Build & Run

Prerequis:
- Rust toolchain (cargo)
- `mdbook` pour construire la documentation (optionnel localement)

Construire la documentation mdBook (depuis le dossier `server`):

```powershell
cd server/doc
mdbook build
```

Pour lancer le serveur localement (port 8080):

```powershell
# depuis la racine du repo
cd server
cargo run --release
```

Créer une instance de simulation via HTTP POST `/api/simulations` puis se connecter en WebSocket sur `/ws?uuid=<uuid>&token=<token>`.

Commit & push de la documentation:

```powershell
git add server/doc
git commit -m "Add generated mdBook documentation for backend"
git push origin feature/documentation
```
