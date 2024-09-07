## Getting Started

Desktop application for

- exploring codeowners of changed files of particular git branch
- determining owners of particular file
- exploring owners for all repository in tree format

## Installation

- Download app installation file with `.dmg` extension [from release page](https://github.com/khludenevav/codeowner-viewer/releases/latest). Note, only M1, M2, M3.. processors supported.
- Install it as usual app
- Execute in any console `xattr -dr com.apple.quarantine /Applications/Codeowners\ viewer.app`.
  It required because app is not signed, because Apple asks to pay 99$ per year for signing: https://disable-gatekeeper.github.io/
- Done. You can launch app

App supports auto update if new version available on application launch. To check updates relaunch it. Update doesn't require to enter any new console commands. Just couple "Yes" clicks.

## Used technologies

Main framework: Tauri.

### Frontend

- Build framework: vite
- File-system router: @tanstack/react-router
- State manager/cache: tanstack/react-query
- Css framework: tailwind
- Headless UI components library: https://ui.shadcn.com/
- Icon library: lucide-react
