import { useEffect, useState } from 'react';
import reactLogo from './assets/react.svg';
import viteLogo from '/vite.svg';
import './App.css';
import { path } from '@tauri-apps/api';

function App() {
  const [count, setCount] = useState(0);
  const [result, setResult] = useState<string[]>([]);
  useEffect(() => {
    Promise.allSettled([
      path.appCacheDir().then(p => `appCacheDir: ${p}`),
      path.appConfigDir().then(p => `appConfigDir: ${p}`),
      path.appDataDir().then(p => `appDataDir: ${p}`),
      path.appLocalDataDir().then(p => `appLocalDataDir: ${p}`),
      path.appLogDir().then(p => `appLogDir: ${p}`),
    ]).then(promisesResult => {
      const res: string[] = [];
      promisesResult.forEach(r => {
        if (r.status === 'fulfilled') {
          res.push(r.value);
        }
      });
      setResult(res);
    });
  }, []);

  return (
    <>
      <div>
        <div>
          {result.map(r => (
            <div key={r}>{r}</div>
          ))}
        </div>
        <a href='https://vitejs.dev' target='_blank'>
          <img src={viteLogo} className='logo' alt='Vite logo' />
        </a>
        <a href='https://react.dev' target='_blank'>
          <img src={reactLogo} className='logo react' alt='React logo' />
        </a>
      </div>
      <h1>Vite + React</h1>
      <div className='card'>
        <button onClick={() => setCount(count => count + 1)}>count is {count}</button>
        <p>
          Edit <code>src/App.tsx</code> and save to test HMR
        </p>
      </div>
      <p className='read-the-docs'>Click on the Vite and React logos to learn more</p>
    </>
  );
}

export default App;
