import { useCallback, useEffect, useRef, useState } from 'react';
import './App.css';
import { type AppConfig, readAppConfig, writeAppConfig, DEFAULT_APP_CONFIG } from './appConfig';
import { open } from '@tauri-apps/api/dialog';
import { path } from '@tauri-apps/api';
import { getBranchDifference } from './codeowners-command';

function App() {
  const branchInputRef = useRef<HTMLInputElement>(null);
  const [isLoading, setIsLoading] = useState<boolean>();
  const [error, setError] = useState<string>('');
  const [lastOwners, setLastOwners] = useState<Map<string, string[]> | null>(null);
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
          { repoPath: selectedDirectory, codeowners: 'CODEOWNERS' },
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

  const onBranchDiff = useCallback(async () => {
    const branch = branchInputRef.current?.value?.trim();
    if (!branch) {
      return;
    }
    const repository = appConfig?.repositories[0];
    if (repository) {
      setLastOwners(null);
      setIsLoading(true);
      try {
        const owners = await getBranchDifference(repository, branch);
        setLastOwners(owners);
      } finally {
        setIsLoading(false);
      }
    } else {
      console.log('No repo');
      setLastOwners(null);
    }
  }, [appConfig?.repositories]);

  if (error) {
    return `Error: ${error}`;
  }

  if (!appConfig) {
    return 'Loading app config...';
  }

  return (
    <>
      <pre>{JSON.stringify(appConfig, null, 2)}</pre>
      <div>
        <button onClick={addRepository}>Add repository</button>
        <button onClick={removeRepository}>Remove repository</button>
        <button onClick={resetEntireAppConfig}>Reset entire app config</button>
      </div>
      <div style={{ height: '16px' }} />
      <input autoFocus style={{ minWidth: '300px' }} type='text' ref={branchInputRef}></input>
      <button style={{ marginLeft: '8px' }} onClick={onBranchDiff}>
        onBranchDiff
      </button>
      <hr />
      {isLoading && <div>Finding codeowners...</div>}
      {lastOwners && !isLoading && (
        <pre>{JSON.stringify(Object.fromEntries(lastOwners.entries()), null, 2)}</pre>
      )}
    </>
  );
}

export default App;
