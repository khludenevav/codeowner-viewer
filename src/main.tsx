import React from 'react';
import ReactDOM from 'react-dom/client';
import { RouterProvider, createRouter } from '@tanstack/react-router';
import './main.css';
// Import the generated route tree
import { routeTree } from './routeTree.gen';
import { focusManager, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ThemeProvider } from './components/theme/theme-provider';
import { TooltipProvider } from './components/ui/tooltip';
import { appWindow } from '@tauri-apps/api/window';
import { TauriEvent } from '@tauri-apps/api/event';

// Create a new router instance
const router = createRouter({ routeTree });

// Register the router instance for type safety
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}

const QUERY_CLIENT = new QueryClient({
  defaultOptions: {
    queries: {
      // By default without network queries doesn't launched. But this app doesn't need network
      networkMode: 'always',
      retry: false,
      retryOnMount: true,
      staleTime: 0,
      refetchInterval: false,
      refetchIntervalInBackground: false,
      refetchOnWindowFocus: true,
    },
  },
});

// React query focus event works incorrectly. So we manually managing focus state of app
appWindow.listen(TauriEvent.WINDOW_FOCUS, () => {
  focusManager.setFocused(true);
});
appWindow.listen(TauriEvent.WINDOW_BLUR, () => {
  focusManager.setFocused(false);
});

// Render the app
const rootElement = document.getElementById('root')!;
if (!rootElement.innerHTML) {
  const root = ReactDOM.createRoot(rootElement);
  root.render(
    <React.StrictMode>
      <QueryClientProvider client={QUERY_CLIENT}>
        <ThemeProvider>
          <TooltipProvider>
            <RouterProvider router={router} />
          </TooltipProvider>
        </ThemeProvider>
      </QueryClientProvider>
    </React.StrictMode>,
  );
}
