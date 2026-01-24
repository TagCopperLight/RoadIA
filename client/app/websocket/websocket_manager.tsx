import {onOpen, receive} from './websocket_listener';

export class WebsocketManager {
    private socket: WebSocket;

    constructor() {
        this.socket = new WebSocket("ws://localhost:8080");

        this.socket.onopen = () => {
            onOpen();
        };

        this.socket.onmessage = this.onMessage;
    }

    private onMessage(event:MessageEvent){
        const json = JSON.parse(event.data);
        const packetID:String = json.PacketID;
        const data = json.Data;

        receive(packetID, data);
    }

    public send(packetID:String, data:any){
        const json = {PacketID:packetID, Data:data};
        this.socket.send(JSON.stringify(json));
    }

}
