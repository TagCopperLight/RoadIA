'use client';

import { WsProvider } from '@/app/websocket/websocket';
import { EditModeProvider } from './EditModeContext';
import MapComponent from './MapComponent';
import Toolbar from './Toolbar';

export default function MapShell({ uuid }: { uuid: string }) {
    return (
        <WsProvider uuid={uuid}>
            <EditModeProvider>
                <Toolbar />
                <div className='flex w-full h-full p-[15px]'>
                    <MapComponent />
                </div>
            </EditModeProvider>
        </WsProvider>
    );
}
