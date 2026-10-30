// rapidraw-web verzije: RapidRAW (src-tauri/tauri.conf.json) + web sloj (rrweb/VERSION, semver).
//   tag / release:   rapidraw-v<rapidraw>-web-v<web>    npr. rapidraw-v1.6.4-web-v1.1.0
//   verzija paketa:  <rapidraw>+web.<web>               npr. 1.6.4+web.1.1.0
// Paketi tako i uz isti RapidRAW svaki web update vide kao noviju verziju (apt, dnf, NSIS).
// Piše rrweb/tauri.version.json (overlay za tauri build) i ispisuje tag.
//   node rrweb/version.mjs [--check <tag>]     --check: web dio taga mora odgovarati rrweb/VERSION
import { readFileSync, writeFileSync } from 'node:fs';

const rapidraw = JSON.parse(readFileSync('src-tauri/tauri.conf.json', 'utf8')).version;
const web = readFileSync('rrweb/VERSION', 'utf8').trim();
if (!/^\d+\.\d+\.\d+$/.test(web)) throw new Error(`rrweb/VERSION: expected X.Y.Z, got "${web}"`);

const i = process.argv.indexOf('--check');
if (i > 0) {
  const tag = process.argv[i + 1] ?? '';
  const m = /^rapidraw-(v.+)-web-v(\d+\.\d+\.\d+)$/.exec(tag);
  if (!m || m[2] !== web) {
    console.error(`${tag}: web version must match rrweb/VERSION (${web})`);
    process.exit(1);
  }
  if (m[1] !== `v${rapidraw}`) console.warn(`warning: ${tag} vs src-tauri/tauri.conf.json version ${rapidraw}`);
}

writeFileSync('rrweb/tauri.version.json', `${JSON.stringify({ version: `${rapidraw}+web.${web}` })}\n`);
console.log(`rapidraw-v${rapidraw}-web-v${web}`);
