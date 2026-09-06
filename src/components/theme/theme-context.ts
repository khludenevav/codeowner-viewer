import { createContext } from 'react';
import { ColorTheme } from '@/app-config/app-config';

export type ThemeProviderState = {
  theme: ColorTheme;
  setTheme: (theme: ColorTheme) => void;
};

const initialState: ThemeProviderState = {
  theme: 'system',
  setTheme: () => null,
};

export const ThemeProviderContext = createContext<ThemeProviderState>(initialState);
