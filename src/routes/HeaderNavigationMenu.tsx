import { useAppConfig } from '@/app-config/useAppConfig';
import { ThemeToggle } from '@/components/theme/ThemeToggle';
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
  navigationMenuTriggerStyle,
} from '@/components/ui/navigation-menu';
import { Tooltip } from '@/components/ui/tooltip';
import { Link, useMatchRoute } from '@tanstack/react-router';
import { SettingsIcon } from 'lucide-react';

export const HeaderNavigationMenu = () => {
  const matchRoute = useMatchRoute();
  const appConfigResponse = useAppConfig();
  const noRepositories =
    appConfigResponse.status === 'success' && appConfigResponse.data.repositories.length === 0;
  return (
    <NavigationMenu className='pb-2 border-b'>
      <NavigationMenuList>
        <Tooltip content={noRepositories ? 'Add at least one repository in settings menu' : null}>
          <NavigationMenuItem>
            <NavigationMenuLink
              className={navigationMenuTriggerStyle()}
              asChild
              active={!!matchRoute({ to: '/repositories/$repositoryId/codeowners' })}
            >
              <Link to='/repositories/$repositoryId/codeowners' params={{ repositoryId: 'any' }}>
                Branch changes
              </Link>
            </NavigationMenuLink>
            <NavigationMenuLink
              className={navigationMenuTriggerStyle()}
              asChild
              active={!!matchRoute({ to: '/repositories/$repositoryId/file-owner' })}
            >
              <Link to='/repositories/$repositoryId/file-owner' params={{ repositoryId: 'any' }}>
                File owners
              </Link>
            </NavigationMenuLink>
            <NavigationMenuLink
              className={navigationMenuTriggerStyle()}
              asChild
              active={!!matchRoute({ to: '/repositories/$repositoryId/all-owners' })}
            >
              <Link to='/repositories/$repositoryId/all-owners' params={{ repositoryId: 'any' }}>
                Repo owners (in development)
              </Link>
            </NavigationMenuLink>
          </NavigationMenuItem>
        </Tooltip>

        <NavigationMenuItem className='ml-auto'>
          <NavigationMenuLink
            className={navigationMenuTriggerStyle()}
            asChild
            active={!!matchRoute({ to: '/settings' })}
          >
            <Link to='/settings'>
              <SettingsIcon className='h-5 w-5' />
              &nbsp;Settings
            </Link>
          </NavigationMenuLink>
        </NavigationMenuItem>

        <NavigationMenuItem>
          <ThemeToggle />
        </NavigationMenuItem>
      </NavigationMenuList>
    </NavigationMenu>
  );
};
