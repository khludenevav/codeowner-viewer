import { Settings as SettingsIcon } from 'lucide-react';
import {
  NavigationMenu,
  NavigationMenuLink,
  NavigationMenuList,
  navigationMenuTriggerStyle,
} from '@/components/ui/navigation-menu';
import { createRootRoute, Link, Outlet } from '@tanstack/react-router';
import { TanStackRouterDevtools } from '@tanstack/router-devtools';
import { Button } from '@/components/ui/button';

export const Route = createRootRoute({
  component: () => (
    <>
      <div className='flex justify-between'>
        <NavigationMenu className='mb-6'>
          <NavigationMenuList>
            <Link to='/settings'>
              <NavigationMenuLink className={navigationMenuTriggerStyle()}>
                Settings
              </NavigationMenuLink>
            </Link>

            <Link to='/repositories/$repositoryId/codeowners' params={{ repositoryId: 'any' }}>
              <NavigationMenuLink className={navigationMenuTriggerStyle()}>
                Repository codeowners
              </NavigationMenuLink>
            </Link>
          </NavigationMenuList>
        </NavigationMenu>
        <div className='flex gap-2'>
          <Button variant='outline' size='icon'>
            <SettingsIcon className='h-5 w-5' />
          </Button>
        </div>
      </div>
      <Outlet />
      <TanStackRouterDevtools />
    </>
  ),
});
