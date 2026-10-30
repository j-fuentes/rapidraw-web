// Release notes za rapidraw-v<RapidRAW>-web-v<web>: rrweb/RELEASE_NOTES.md + naslov upstream releasea + što je novo:
// sekcija "## <web>" iz rrweb/CHANGELOG.md, a ako je nema, commitovi web sloja od prethodne web verzije. Izvorni commit sloja
// svakog taga je u poruci taga (main@<sha>, piše ga rrweb-sync.yml). Treba punu git povijest (fetch-depth: 0).
//   node rrweb/release-notes.mjs rapidraw-v1.6.4-web-v1.1.0 > notes.md      (GH_TOKEN opcionalno, za GitHub API)
import { execFileSync } from 'node:child_process';
import { readFileSync } from 'node:fs';

const TAG_RE = /^rapidraw-(v.+)-web-v(\d+\.\d+\.\d+)$/;
const tag = process.argv[2] ?? '';
if (!TAG_RE.test(tag)) throw new Error('usage: node rrweb/release-notes.mjs rapidraw-vX.Y.Z-web-vA.B.C');
const [, up, web] = TAG_RE.exec(tag);
const REPO = 'https://github.com/vedranius/rapidraw-web';
const LAYER = ['rrweb', '.github/README.md', '.github/workflows/rrweb-release.yml', '.github/workflows/rrweb-sync.yml',
  ':(exclude)rrweb/VERSION']; // commit koji samo podiže verziju nije promjena
const git = (...args) => execFileSync('git', args, { encoding: 'utf8' }).trim();
const cmp = (a, b) => { const x = a.split('.').map(Number), y = b.split('.').map(Number); return x[0] - y[0] || x[1] - y[1] || x[2] - y[2]; };
const source = (t) => /main@([0-9a-f]{7,40})/.exec(git('tag', '--list', '--format=%(contents)', t))?.[1] ?? t;
const changesIn = (range) => git('log', '--no-merges', '--format=- %s', ...range, '--', ...LAYER);

let title = `RapidRAW ${up}`;
try {
  const r = await fetch(`https://api.github.com/repos/CyberTimon/RapidRAW/releases/tags/${up}`,
    { headers: process.env.GH_TOKEN ? { authorization: `Bearer ${process.env.GH_TOKEN}` } : {} });
  if (r.ok) title = (await r.json()).name || title;
} catch { /* bez naslova, ostaje "RapidRAW vX.Y.Z" */ }

const releases = git('tag', '--list', 'rapidraw-v*-web-v*').split('\n').filter((t) => TAG_RE.test(t) && t !== tag)
  .map((t) => ({ tag: t, up: TAG_RE.exec(t)[1], web: TAG_RE.exec(t)[2] }));
const byUp = (a, b) => a.up.localeCompare(b.up, undefined, { numeric: true });
// ista web verzija već objavljena na starijem RapidRAW-u → ovo je samo novi RapidRAW
const same = releases.filter((r) => r.web === web && byUp(r, { up }) < 0).sort(byUp).pop();
const older = releases.filter((r) => cmp(r.web, web) < 0).sort((a, b) => cmp(b.web, a.web))[0];
const legacy = git('tag', '--list', 'v*-web', '--sort=-v:refname').split('\n').filter(Boolean)[0]; // stara shema vX.Y.Z-web

// ručno napisana sekcija u CHANGELOG-u ima prednost pred popisom commitova
const changelog = (() => {
  try {
    const m = readFileSync('rrweb/CHANGELOG.md', 'utf8').split(/^## /m).find((sec) => sec.startsWith(`${web}\n`) || sec.startsWith(`${web} `));
    return m?.slice(m.indexOf('\n') + 1).trim() || null;
  } catch { return null; }
})();

let changes;
if (same) {
  changes = `- Same web layer as [${same.tag}](${REPO}/releases/tag/${same.tag}); this release brings RapidRAW ${up}.`;
} else if (changelog) {
  changes = `${changelog}\n\nFull history: [CHANGELOG](https://github.com/vedranius/rapidraw-web/blob/main/rrweb/CHANGELOG.md)`;
} else if (older) {
  changes = changesIn([`${source(older.tag)}..${source(tag)}`]) || '- Maintenance release.';
} else if (legacy) {
  changes = changesIn([`--since=${git('log', '-1', '--format=%cI', legacy)}`, source(tag)]) || '- Maintenance release.';
} else {
  changes = '- First rapidraw-web release.';
}

process.stdout.write(readFileSync('rrweb/RELEASE_NOTES.md', 'utf8')
  .replaceAll('{{UPSTREAM_TAG}}', () => up)
  .replaceAll('{{UPSTREAM_TITLE}}', () => title)
  .replaceAll('{{WEB_VERSION}}', () => web)
  .replaceAll('{{PKG_VERSION}}', () => `${up.replace(/^v/, '')}+web.${web}`)
  .replaceAll('{{RRWEB_CHANGES}}', () => changes));
