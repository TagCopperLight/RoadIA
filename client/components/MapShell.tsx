'use client';

import { WsProvider } from '@/app/websocket/websocket';
import { EditModeProvider } from './EditModeContext';
import MapComponent from './MapComponent';
import Toolbar from './Toolbar';
import { ReactNode } from 'react';

export default function MapShell({ uuid, children }: { uuid: string, children?: ReactNode }) {
    return (
        <WsProvider uuid={uuid}>
            <EditModeProvider>
                {children}
                <Toolbar />
                <div className='flex w-full h-full p-[15px]'>
                    <MapComponent />
                </div>
            </EditModeProvider>
        </WsProvider>
    );
}
