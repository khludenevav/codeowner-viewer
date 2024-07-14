import { ColorTheme, DEFAULT_COLOR_THEME } from '@/app-config/app-config';
import { useAppConfig, useUpdateAppConfig } from '@/app-config/useAppConfig';
import { createContext, useEffect, useMemo } from 'react';

type ThemeProviderProps = {
  children: React.ReactNode;
  defaultTheme?: ColorTheme;
};

type ThemeProviderState = {
  theme: ColorTheme;
  setTheme: (theme: ColorTheme) => void;
};

const initialState: ThemeProviderState = {
  theme: 'system',
  setTheme: () => null,
};

export const ThemeProviderContext = createContext<ThemeProviderState>(initialState);

export function ThemeProvider({ children, ...props }: ThemeProviderProps) {
  const appConfigResponse = useAppConfig();
  const appConfigUpdate = useUpdateAppConfig();

  const theme =
    appConfigResponse.status === 'success'
      ? appConfigResponse.data.theme.colorTheme
      : DEFAULT_COLOR_THEME;

  useEffect(() => {
    if (appConfigResponse.status !== 'success') {
      return;
    }
    const root = window.document.documentElement;

    root.classList.remove('light', 'dark');

    if (theme === 'system') {
      const systemTheme = window.matchMedia('(prefers-color-scheme: dark)').matches
        ? 'dark'
        : 'light';

      root.classList.add(systemTheme);
    } else {
      root.classList.add(theme);
    }
  }, [appConfigResponse.status, theme]);

  const value = useMemo(() => {
    return {
      theme,
      setTheme: (colorTheme: ColorTheme) => {
        if (appConfigResponse.status === 'success') {
          appConfigUpdate.mutate({
            appConfig: {
              ...appConfigResponse.data,
              theme: {
                ...appConfigResponse.data.theme,
                colorTheme,
              },
            },
          });
        }
      },
    };
  }, [appConfigResponse.data, appConfigResponse.status, appConfigUpdate, theme]);

  return (
    <ThemeProviderContext.Provider {...props} value={value}>
      {children}
    </ThemeProviderContext.Provider>
  );
}
