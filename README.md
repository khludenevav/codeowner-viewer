# Installation

Download latest dmg file (like Codeowners.viewer_0.1.3_aarch64.dmg) from releases page
from https://github.com/khludenevav/codeowner-viewer/releases

Double click it and install to applications. Application is not signed, so you have to exclude it from quarantine by executing `xattr -dr com.apple.quarantine /Applications/Codeowners\ viewer.app`. Now you can use it.

More info about "pay Apple 99$" per year here: https://disable-gatekeeper.github.io/

# Libraries

## Frontend

- Main framework: vite
- File-system router: @tanstack/react-router
- State manager/cache: tanstack/react-query
- Css framework: tailwind
- Headless UI components library: https://ui.shadcn.com/
- Icon library: lucide-react

## Assert

- Import from repo/public: `import viteLogo from '/vite.svg';`
- Import from repo/src/assets: `import reactLogo from './assets/react.svg';`

## Allowlist

tauri allow list justification:
`$DATA/*` in the `"scope": ["$DATA/*", "$APPCONFIG/*"],` added to be able to make directory for application. By
`await createDir('', { dir: BaseDirectory.AppConfig, recursive: true });`

## Expanding the ESLint configuration

If you are developing a production application, we recommend updating the configuration to enable type aware lint rules:

- Configure the top-level `parserOptions` property like this:

```js
export default {
  // other rules...
  parserOptions: {
    ecmaVersion: 'latest',
    sourceType: 'module',
    project: ['./tsconfig.json', './tsconfig.node.json'],
    tsconfigRootDir: __dirname,
  },
};
```

- Replace `plugin:@typescript-eslint/recommended` to `plugin:@typescript-eslint/recommended-type-checked` or `plugin:@typescript-eslint/strict-type-checked`
- Optionally add `plugin:@typescript-eslint/stylistic-type-checked`
- Install [eslint-plugin-react](https://github.com/jsx-eslint/eslint-plugin-react) and add `plugin:react/recommended` & `plugin:react/jsx-runtime` to the `extends` list
