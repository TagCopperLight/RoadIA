import {WebsocketManager} from "./websocket_manager";

const websocketManager:WebsocketManager = new WebsocketManager();

// Envoie un token au serveur prouvant l'identité du client
export function sendConnectionToken(token:String){
    const data = {Token:token};
    websocketManager.send("connect", data);
}