import ignore from 'ignore';

type CodeownerLine = {
  line: number;
  pattern: string;
  owners: string[];
  required: boolean;
};

function filterCodeownerLine(line: CodeownerLine | null): line is CodeownerLine {
  return !!line && line.owners.length > 0;
}

type Codeowners = CodeownerLine[];

export function parseCodeowners(codeownersFileContent: string): Codeowners {
  const lines: CodeownerLine[] = codeownersFileContent
    .split('\n')
    .map((line, index) => {
      const [declarations, comments] = line.split('#').map(x => x.trim());
      if (!declarations?.trim()) {
        return null;
      }
      const required = !comments?.includes('!required');
      const [pattern, ...owners] = declarations.split(/\s+/).map(x => x.trim());
      return { line: index + 1, pattern, owners, required };
    })
    .filter(filterCodeownerLine)
    .reverse();
  return lines;
}

/** filename argument here is relative path from repo */
export function getOwnerTeam({
  codeowners,
  filename,
}: {
  codeowners: Codeowners;
  filename: string;
}) {
  for (const entry of codeowners) {
    if (ignore().add(entry.pattern).ignores(filename)) {
      return entry.owners.join(', ');
    }
  }
  return '';
}
