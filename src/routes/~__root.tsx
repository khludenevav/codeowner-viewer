import { createRootRoute, Outlet } from '@tanstack/react-router';
import { HeaderNavigationMenu } from './HeaderNavigationMenu';
import React from 'react';
import { AppUpdater } from './AppUpdater';

const TanStackRouterDevtools =
  process.env.NODE_ENV === 'production'
    ? () => null
    : React.lazy(() =>
        import('@tanstack/router-devtools').then(res => ({
          default: res.TanStackRouterDevtools,
        })),
      );

export const Route = createRootRoute({
  component: () => (
    <>
      <div className='max-h-dvh overflow-y-hidden flex flex-col'>
        <header className='px-6 pt-6'>
          <HeaderNavigationMenu />
        </header>
        <main className='flex-1 overflow-y-auto overscroll-none' id='main'>
          <Outlet />
        </main>
      </div>
      <AppUpdater />
      <React.Suspense>
        <TanStackRouterDevtools />
      </React.Suspense>
    </>
  ),
});
