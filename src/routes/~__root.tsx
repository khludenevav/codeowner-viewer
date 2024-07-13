import { createRootRoute, Link, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/router-devtools';
import './~__root.css';

export const Route = createRootRoute({
  component: () => (
    <>
      <div>
        <Link to='/'>Home</Link>
        <Link to='/settings'>Settings</Link>
        <Link to='/repositories/$repositoryId/codeowners' params={{ repositoryId: 'any' }}>
          Repository codeowners
        </Link>
      </div>
      <hr />
      <Outlet />
      <TanStackRouterDevtools />
    </>
  ),
});
