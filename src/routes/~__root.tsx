import { createRootRoute, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/router-devtools';
import { HeaderNavigationMenu } from './HeaderNavigationMenu';

export const Route = createRootRoute({
  component: () => (
    <>
      <header className='px-6 mb-6 mt-6'>
        <HeaderNavigationMenu />
      </header>
      <main className='mx-6 mb-6'>
        <Outlet />
      </main>
      <TanStackRouterDevtools />
    </>
  ),
});
