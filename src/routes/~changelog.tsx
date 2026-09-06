import { createFileRoute } from '@tanstack/react-router';
import { History } from 'lucide-react';
// The CHANGELOG lives at the repo root; Vite inlines it as a string at build time.
import changelogRaw from '../../CHANGELOG.md?raw';

export const Route = createFileRoute('/changelog')({
  component: ChangelogPage,
});

type ChangelogEntry = {
  version: string;
  heading: string;
  bullets: string[];
  paragraphs: string[];
};

function parseChangelog(raw: string): { intro: string[]; entries: ChangelogEntry[] } {
  // Strip HTML comments (used in CHANGELOG.md for maintainer-only notes).
  const cleaned = raw.replace(/<!--[\s\S]*?-->/g, '');
  const lines = cleaned.split(/\r?\n/);
  const entries: ChangelogEntry[] = [];
  const intro: string[] = [];
  let current: ChangelogEntry | null = null;

  for (const rawLine of lines) {
    const line = rawLine.replace(/\s+$/, '');
    const versionMatch = /^##\s+(.+)$/.exec(line);
    if (versionMatch) {
      if (current) entries.push(current);
      const heading = versionMatch[1].trim();
      const version = heading.split(/\s+/)[0] ?? heading;
      current = { version, heading, bullets: [], paragraphs: [] };
      continue;
    }
    if (/^#\s+/.test(line)) {
      continue;
    }
    if (!current) {
      // Everything before the first `## version` heading is intro copy.
      if (line.trim().length > 0) {
        intro.push(line.trim());
      }
      continue;
    }
    const bulletMatch = /^\s*[-*]\s+(.+)$/.exec(line);
    if (bulletMatch) {
      current.bullets.push(bulletMatch[1].trim());
      continue;
    }
    if (line.trim().length === 0) {
      continue;
    }
    if (current.bullets.length > 0) {
      // Continuation of the previous bullet (indented wrapped line).
      current.bullets[current.bullets.length - 1] += ' ' + line.trim();
    } else {
      current.paragraphs.push(line.trim());
    }
  }
  if (current) entries.push(current);
  return { intro, entries };
}

function ChangelogPage() {
  const { intro, entries } = parseChangelog(changelogRaw);
  return (
    <div className='mx-auto flex max-w-4xl flex-col gap-8 px-6 py-8'>
      <header className='flex flex-col gap-2'>
        <div className='flex items-center gap-3'>
          <History className='h-6 w-6 flex-shrink-0' />
          <h1 className='text-2xl font-semibold'>Changelog</h1>
        </div>
        {intro.length > 0 && <p className='text-sm text-muted-foreground'>{intro.join(' ')}</p>}
      </header>

      {entries.length === 0 ? (
        <p className='text-sm text-muted-foreground'>No release notes yet.</p>
      ) : (
        <div className='flex flex-col gap-6'>
          {entries.map(entry => (
            <section key={entry.version} className='rounded-md border border-border bg-card p-4'>
              <h2 className='text-lg font-semibold'>{entry.heading}</h2>
              {entry.paragraphs.length > 0 && (
                <div className='mt-2 flex flex-col gap-2 text-sm'>
                  {entry.paragraphs.map((paragraph, i) => (
                    <p key={i}>{paragraph}</p>
                  ))}
                </div>
              )}
              {entry.bullets.length > 0 && (
                <ul className='mt-2 list-disc pl-6 text-sm'>
                  {entry.bullets.map((bullet, i) => (
                    <li key={i}>{bullet}</li>
                  ))}
                </ul>
              )}
            </section>
          ))}
        </div>
      )}
    </div>
  );
}
