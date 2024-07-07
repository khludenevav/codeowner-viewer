import { Command } from '@tauri-apps/api/shell';
import { getOwnerTeam, parseCodeowners } from './codeowners-utils';
import { path } from '@tauri-apps/api';
import { Repositories } from './appConfig';
import { invoke } from '@tauri-apps/api';

export async function getBranchDifference(
  repository: Repositories,
  branch: string,
): Promise<null | Map<string, string[]>> {
  const command = new Command(
    'run-git-command',
    ['--no-pager', 'diff', '--name-only', `origin/main...${branch}`],
    { cwd: repository.repoPath },
  );
  const output = await command.execute();
  if (output.code !== 0) {
    console.log(`Execution code: ${output.code}.\n${output.stderr}`);
    return null;
  }
  const lines = output.stdout.split('\n');

  const fullCodeownersFilePath = await path.join(repository.repoPath, repository.codeowners);
  const codeownersFileContent: string = await invoke('get_codeowners_content', {
    codeownersPath: fullCodeownersFilePath,
  });
  const codeowners = parseCodeowners(codeownersFileContent);

  const owners = new Map<string, string[]>();
  for (const filePath of lines) {
    if (!filePath) {
      continue;
    }
    const ownerTeam = getOwnerTeam({ codeowners, filename: filePath });
    let teamFiles = owners.get(ownerTeam);
    if (!teamFiles) {
      teamFiles = [];
      owners.set(ownerTeam, teamFiles);
    }
    teamFiles.push(filePath);
  }
  return owners;
}
