// Appelé lorsque la connexion websocket est ouverte
export function onOpen(){
}

// Appelé à chaque nouveau message reçu
export function receive(packetID:String, data:any){
    switch (packetID) {
        case "map":
            loadMap(data);
            break;
        case "cars":
            updateCars(data);
            break;
        default:
            console.error(`[ERROR] PacketID : \"${packetID}\" not defined`);
    }
}

// Charge la map
function loadMap(json:any) {
    // to do
    console.log(json);
}

// Met à jour la position des voitures lors de la simulation
function updateCars(json:any) {
    // to do
    console.log(json);
}