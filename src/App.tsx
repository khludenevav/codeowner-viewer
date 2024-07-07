import { useCallback, useEffect, useState } from 'react';
import './App.css';
import { type AppConfig, readAppConfig, writeAppConfig, DEFAULT_APP_CONFIG } from './appConfig';
import { open } from '@tauri-apps/api/dialog';
import { path } from '@tauri-apps/api';

function App() {
  const [error, setError] = useState<string>('');
  const [appConfig, setAppConfig] = useState<AppConfig | null>(null);
  useEffect(() => {
    readAppConfig()
      .then(config => setAppConfig(config))
      .catch(error => setError(error));
  }, []);

  const addRepository = useCallback(async () => {
    const selectedDirectory = await open({
      title: 'Select repository directory',
      defaultPath: await path.homeDir(),
      directory: true,
      recursive: true,
    });
    if (!selectedDirectory || Array.isArray(selectedDirectory)) {
      return;
    }
    setAppConfig(prevAppConfig => {
      const newAppConfig = {
        ...prevAppConfig,
        repositories: [
          ...prevAppConfig!.repositories,
          { repoPart: selectedDirectory, codeowners: 'CODEOWNERS' },
        ],
      };
      writeAppConfig(newAppConfig);
      return newAppConfig;
    });
  }, []);

  const removeRepository = useCallback(async () => {
    setAppConfig(prevAppConfig => {
      const repositories = prevAppConfig!.repositories;
      repositories.pop();
      const newAppConfig = {
        ...prevAppConfig,
        repositories: repositories,
      };
      writeAppConfig(newAppConfig);
      return newAppConfig;
    });
  }, []);

  const resetEntireAppConfig = useCallback(async () => {
    setAppConfig(() => {
      const newAppConfig = DEFAULT_APP_CONFIG;
      writeAppConfig(newAppConfig);
      return newAppConfig;
    });
  }, []);

  if (error) {
    return `Error: ${error}`;
  }

  if (!appConfig) {
    return 'Loading app config...';
  }

  return (
    <>
      <pre>{JSON.stringify(appConfig, null, 2)}</pre>
      <button onClick={addRepository}>Add repository</button>
      <button onClick={removeRepository}>Remove repository</button>
      <button onClick={resetEntireAppConfig}>Reset entire app config</button>
    </>
  );
}

export default App;
