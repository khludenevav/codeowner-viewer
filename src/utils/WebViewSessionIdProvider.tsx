import React, { createContext, useContext, useState } from 'react';

function generateStableId(): string {
  // Combines current time with a random number for uniqueness
  return Date.now().toString(36) + '-' + Math.random().toString(36).substr(2, 9);
}

const WebViewSessionIdContext = createContext<string | undefined>(undefined);

/**
 * When user clicks Reload in context menu, the old commands on BE still can work.
 * So we need to listen to only this session ID. It solves the problem on UI,
 * but BE still calculates the data which no one needs, we accept it.
 */
export const WebViewSessionIdProvider: React.FC<{ children: React.ReactNode }> = ({ children }) => {
  const [sessionId] = useState(generateStableId);
  return (
    <WebViewSessionIdContext.Provider value={sessionId}>
      {children}
    </WebViewSessionIdContext.Provider>
  );
};

// eslint-disable-next-line react-refresh/only-export-components
export function useWebViewSessionId() {
  const context = useContext(WebViewSessionIdContext);
  if (context === undefined) {
    throw new Error('useWebViewSessionId must be used within a WebViewSessionIdProvider');
  }
  return context;
}
