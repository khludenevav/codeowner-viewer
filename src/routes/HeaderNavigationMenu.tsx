import { ThemeToggle } from '@/components/theme/ThemeToggle';
import {
  NavigationMenu,
  NavigationMenuItem,
  NavigationMenuLink,
  NavigationMenuList,
  navigationMenuTriggerStyle,
} from '@/components/ui/navigation-menu';
import { Link, useMatchRoute } from '@tanstack/react-router';
import { SettingsIcon } from 'lucide-react';

export const HeaderNavigationMenu = () => {
  const matchRoute = useMatchRoute();
  return (
    <NavigationMenu className='pb-2 border-b'>
      <NavigationMenuList>
        <NavigationMenuItem>
          <NavigationMenuLink
            className={navigationMenuTriggerStyle()}
            asChild
            active={!!matchRoute({ to: '/repositories/$repositoryId/codeowners' })}
          >
            <Link to='/repositories/$repositoryId/codeowners' params={{ repositoryId: 'any' }}>
              Repository codeowners
            </Link>
          </NavigationMenuLink>
        </NavigationMenuItem>

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
