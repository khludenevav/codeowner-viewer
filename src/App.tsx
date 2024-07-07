import { useEffect, useState } from 'react';
import './App.css';
import { type AppConfig, readAppConfig } from './appConfig';

function App() {
  const [error, setError] = useState<string>('');
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  useEffect(() => {
    readAppConfig()
      .then(config => setAppConfig(config))
      .catch(error => setError(error));
  }, []);

  if (error) {
    return `Error: ${error}`;
  }

  if (!appConfig) {
    return 'Loading app config...';
  }

  return (
    <>
      <div>{JSON.stringify(appConfig)}</div>
    </>
  );
}

export default App;
