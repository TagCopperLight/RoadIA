'use client';

import { WsProvider } from '@/app/websocket/websocket';
import MapComponent from './MapComponent';
import Toolbar from './Toolbar';

export default function MapShell({ uuid }: { uuid: string }) {
    return (
        <WsProvider uuid={uuid}>
            <Toolbar />
            <div className='flex w-full h-full pl-[15px] pr-[15px] pt-[15px] pb-[15px]'>
                <MapComponent />
            </div>
        </WsProvider>
    );
}
