# RoadIA
Traffic simulation

# Run the Front-end
To run the frontend in development mode, execute these commands :  
`git clone git@github.com:TagCopperLight/RoadIA.git`  
`cd RoadIA/client/`  
`npm install`  
`npm run dev`  
Then, open http://localhost:3000 in your favorite browser.

For now the maps are in http://localhost:3000/map/{uuid} where {uuid} is not used and can be any string.

# Run the Backend
The backend lives in `server/` and exposes `POST /api/simulations` and the WebSocket endpoint `/ws` on port `8080`.
It expects `ALLOWED_ORIGINS` to be set as a comma-separated list of allowed frontend origins.

# Simulation Duration
Simulations now run with a maximum simulated duration of one day: `MAX_DURATION = 86400` seconds.
The maximum number of steps is derived from that duration and the simulation step size.