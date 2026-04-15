<!-- WebSocket JSON examples for ClientPacket and ServerPacket -->

# WebSocket Examples

Ce fichier rassemble des exemples JSON envoyés et reçus via le WebSocket.

## Format général

Les paquets utilisent `serde` avec `tag = "id"` et `content = "data"`, `rename_all = "camelCase"`.

Exemple générique:

```json
{ "id": "AddNode", "data": { "x": 12.3, "y": 45.6, "kind": "Intersection" } }
```

---

## Client → Serveur (ClientPacket) exemples

- Start / Stop / Reset:

```json
{ "id": "StartSimulation", "data": {} }
{ "id": "StopSimulation", "data": {} }
{ "id": "ResetSimulation", "data": {} }
```

- Edition de nœud:

```json
{ "id": "AddNode", "data": { "x": 10.0, "y": 5.0, "kind": "Intersection" } }
{ "id": "DeleteNode", "data": { "id": 17 } }
{ "id": "MoveNode", "data": { "id": 17, "x": 12.0, "y": 6.0 } }
{ "id": "UpdateNode", "data": { "id": 17, "kind": "Workplace" } }
```

- Edition de route:

```json
{ "id": "AddRoad", "data": { "fromId": 3, "toId": 4, "laneCount": 2, "speedLimit": 13.9 } }
{ "id": "DeleteRoad", "data": { "id": 21 } }
{ "id": "UpdateRoad", "data": { "id": 21, "speedLimit": 11.1 } }
```

Note: les clés internes suivent `camelCase` à la sérialisation (par ex. `laneCount`, `speedLimit`).

---

## Serveur → Client (ServerPacket) exemples

- `Map` (extrait de `serialize_map`):

```json
{
  "id": "Map",
  "data": {
    "nodes": [
      { "id": 1, "kind": "Intersection", "x": 10.0, "y": 5.0, "has_traffic_light": false, "radius": 4.0 }
    ],
    "edges": [
      { "id": 2, "from": 1, "to": 3, "lane_count": 2, "lane_width": 3.0, "length": 120.0, "speed_limit": 13.9 }
    ]
  }
}
```

- `VehicleUpdate`:

```json
{
  "id": "VehicleUpdate",
  "data": {
    "vehicles": [
      {
        "id": 5,
        "x": 11.2,
        "y": 6.3,
        "heading": 1.5708,
        "kind": "Car",
        "state": "Moving"
      }
    ],
    "traffic_lights": [ { "id": 1, "green_road_ids": [2] } ]
  }
}
```

- `MapEdit` (success / failure):

```json
{
  "id": "MapEdit",
  "data": {
    "success": true,
    "error": null,
    "nodes": [ { "id": 1, "kind": "Habitation", "x": 0, "y": 0, "has_traffic_light": false, "radius": 3.0 } ],
    "edges": [ { "id": 2, "from": 1, "to": 3, "lane_count": 2, "lane_width": 3.0, "length": 120.0, "speed_limit": 13.9 } ]
  }
}

{
  "id": "MapEdit",
  "data": { "success": false, "error": "Stop simulation before editing the map", "nodes": [], "edges": [] }
}
```

- `Score`:

```json
{
  "id": "Score",
  "data": {
    "score": 123.4,
    "totalTripTime": 456.7,
    "totalEmittedCo2": 12.3,
    "networkLength": 789.0,
    "totalDistanceTraveled": 345.6,
    "successRate": 0.92
  }
}
```

---

Usage: copiez ces exemples dans la console WebSocket de votre client (ou dans l'outil `WebSocket` du navigateur) pour simuler interactions.
