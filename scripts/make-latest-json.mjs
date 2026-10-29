// \u751f\u6210 Tauri \u66f4\u65b0\u63cf\u8ff0\u6587\u4ef6 latest.json
//
// \u7528\u6cd5\uff1a
//   node scripts/make-latest-json.mjs                       # \u81ea\u52a8\u4ece target/release/bundle/nsis \u53d6\u5305
//   node scripts/make-latest-json.mjs --notes "\u4fee\u4e86\u4e00\u4e9b bug" --out latest.json
//
// \u9700\u6c42\uff1a\u5df2\u6253\u5305\u4e14\u751f\u6210 .sig\uff08\u6253\u5305\u65f6\u8bbe\u7f6e TAURI_SIGNING_PRIVATE_KEY\uff09

import { readFileSync, writeFileSync, readdirSync, existsSync } from 'node:fs'
import { join, dirname } from 'node:path'
import { fileURLToPath } from 'node:url'
import { execSync } from 'node:child_process'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const args = process.argv.slice(2)
const getArg = (name, fallback) => {
	const i = args.indexOf(name)
	return i !== -1 && args[i + 1] ? args[i + 1] : fallback
}

const pkg = JSON.parse(readFileSync(join(root, 'apps', 'app-frontend', 'package.json'), 'utf8'))
const version = getArg('--version', pkg.version)
const notes = getArg('--notes', `AirCore ${version}`)

// \u4ece git remote \u63a8\u65ad GitHub \u4ed3\u5e93
let repo = getArg('--repo', '')
if (!repo) {
	try {
		const remote = execSync('git remote get-url origin', { cwd: root, encoding: 'utf8' }).trim()
		const m = remote.match(/github\.com[/:]([^/]+)\/([^/.]+)(?:\.git)?$/)
		if (m) repo = `${m[1]}/${m[2]}`
	} catch {
		// ignore
	}
}
if (!repo) {
	console.error('[latest] \u65e0\u6cd5\u63a8\u65ad GitHub \u4ed3\u5e93\uff0c\u8bf7\u7528 --repo owner/name \u6307\u5b9a')
	process.exit(1)
}

const bundleDir = join(root, 'target', 'release', 'bundle', 'nsis')
if (!existsSync(bundleDir)) {
	console.error('[latest] \u627e\u4e0d\u5230\u6253\u5305\u76ee\u5f55\uff1a' + bundleDir)
	console.error('[latest] \u8bf7\u5148\u5728 apps/app \u76ee\u5f55\u6267\u884c\uff1apnpm build\uff08\u9700\u8981\u8bbe\u7f6e TAURI_SIGNING_PRIVATE_KEY\uff09')
	process.exit(1)
}

const exe = readdirSync(bundleDir).find((f) => f.endsWith('-setup.exe') && f.includes(version))
if (!exe) {
	console.error(`[latest] \u5728 ${bundleDir} \u91cc\u6ca1\u627e\u5230\u5305\u542b ${version} \u7684 -setup.exe`)
	console.error('[latest] \u5f53\u524d\u76ee\u5f55\u6587\u4ef6\uff1a' + readdirSync(bundleDir).join(', '))
	process.exit(1)
}

const sigPath = join(bundleDir, exe + '.sig')
if (!existsSync(sigPath)) {
	console.error('[latest] \u7f3a\u5c11\u7b7e\u540d\u6587\u4ef6\uff1a' + sigPath)
	console.error('[latest] \u6253\u5305\u65f6\u9700\u8981\u8bbe\u7f6e TAURI_SIGNING_PRIVATE_KEY \u73af\u5883\u53d8\u91cf')
	process.exit(1)
}

const signature = readFileSync(sigPath, 'utf8').trim()
const url = `https://github.com/${repo}/releases/download/v${version}/${encodeURIComponent(exe)}`

const latest = {
	version,
	notes,
	pub_date: new Date().toISOString().replace(/\.\d{3}Z$/, 'Z'),
	platforms: {
		'windows-x86_64': { url, signature },
	},
}

const out = getArg('--out', join(root, 'latest.json'))
writeFileSync(out, JSON.stringify(latest, null, 2) + '\n', 'utf8')
console.log('[latest] \u5df2\u751f\u6210\uff1a' + out)
console.log('[latest] version = ' + version)
console.log('[latest] url     = ' + url)
