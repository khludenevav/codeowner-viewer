import { createRootRoute, Outlet } from '@tanstack/react-router';
import { HeaderNavigationMenu } from './HeaderNavigationMenu';
import React from 'react';

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
      <header className='px-6 mb-6 mt-6'>
        <HeaderNavigationMenu />
      </header>
      <main className='mx-6 mb-6'>
        <Outlet />
      </main>
      <React.Suspense>
        <TanStackRouterDevtools />
      </React.Suspense>
    </>
  ),
});
