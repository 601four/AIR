// \u7c7b\u578b\u68c0\u67e5\u56de\u5f52\u95e8\u7981\uff1a
// \u8fd0\u884c vue-tsc\uff0c\u5c06\u5f53\u524d\u9519\u8bef\u4e0e\u57fa\u7ebf\u5bf9\u6bd4\uff0c\u53ea\u62a5\u300c\u65b0\u589e\u300d\u7684\u7c7b\u578b\u9519\u8bef\u3002
//
// \u7528\u6cd5\uff1a
//   node scripts/typecheck-regression.mjs            # \u68c0\u67e5\u662f\u5426\u6709\u65b0\u589e\u9519\u8bef
//   node scripts/typecheck-regression.mjs --update   # \u91cd\u65b0\u751f\u6210\u57fa\u7ebf
//
// \u4e3a\u4ec0\u4e48\u8fd9\u4e48\u505a\uff1a
//   1. vite build \u53ea\u8f6c\u8bd1\u4e0d\u505a\u7c7b\u578b\u68c0\u67e5\uff0c\u8fd0\u884c\u671f\u624d\u4f1a\u5d29\uff08\u4f8b\u5982 ref \u672a\u5bfc\u5165\uff09
//   2. \u4ed3\u5e93\u5df2\u6709 1600+ \u5b58\u91cf\u7c7b\u578b\u9519\u8bef\uff0c\u4e0d\u5229\u7528\u5b83\u4eec\u5c31\u65e0\u6cd5\u4f7f\u7528\u68c0\u67e5
//   3. \u57fa\u7ebf\u5f0f\u6bd4\u5bf9\u53ea\u7ba1\u65b0\u5f15\u5165\u7684\u9519\u8bef\uff0c\u4fdd\u8bc1\u4e0d\u4f1a\u8d8a\u6539\u8d8a\u574f

import { spawnSync } from 'node:child_process'
import { readFileSync, writeFileSync, existsSync } from 'node:fs'
import { dirname, join } from 'node:path'
import { fileURLToPath } from 'node:url'

const root = join(dirname(fileURLToPath(import.meta.url)), '..')
const baselinePath = join(root, 'scripts', 'typecheck-baseline.json')
const update = process.argv.includes('--update')

const vueTsc = process.platform === 'win32' ? 'vue-tsc.cmd' : 'vue-tsc'
const bin = join(root, 'node_modules', '.bin', vueTsc)

const result = spawnSync(bin, ['--noEmit', '-p', 'tsconfig.app.json'], {
	cwd: join(root, 'apps', 'app-frontend'),
	encoding: 'utf8',
	shell: process.platform === 'win32',
	maxBuffer: 64 * 1024 * 1024,
})

const output = (result.stdout ?? '') + (result.stderr ?? '')
const lineRe = /^(.+?)\((\d+),(\d+)\): error (TS\d+): (.*)$/
const errors = []
for (const line of output.split(/\r?\n/)) {
	const m = line.trimEnd().match(lineRe)
	if (!m) continue
	const [, file, lineNo, col, code, message] = m
	// \u5ffd\u7565\u6765\u81ea\u4f9d\u8d56\u5305\u4e0e\u5e93\u6587\u4ef6\u7684\u9519\u8bef\uff08\u4e0d\u5f52\u6211\u4eec\u7ba1\uff09
	if (file.includes('node_modules')) continue
	errors.push({ file, line: Number(lineNo), col: Number(col), code, message })
}

const keyOf = (e) => `${e.file}|${e.code}|${e.message.replace(/\s+/g, ' ').trim()}`

if (update) {
	const baseline = [...new Set(errors.map(keyOf))].sort()
	writeFileSync(baselinePath, JSON.stringify(baseline, null, 2) + '\n', 'utf8')
	console.log(`[typecheck] \u57fa\u7ebf\u5df2\u66f4\u65b0\uff1a${baseline.length} \u6761\u5df2\u77e5\u7c7b\u578b\u9519\u8bef`)
	process.exit(0)
}

if (!existsSync(baselinePath)) {
	console.error('[typecheck] \u7f3a\u5c11\u57fa\u7ebf\u6587\u4ef6 scripts/typecheck-baseline.json')
	console.error('[typecheck] \u8bf7\u5148\u8fd0\u884c\uff1anode scripts/typecheck-regression.mjs --update')
	process.exit(2)
}

const baseline = new Set(JSON.parse(readFileSync(baselinePath, 'utf8')))
const baselineFiles = new Set([...baseline].map((k) => k.split('|')[0]))
const newErrors = errors.filter((e) => !baseline.has(keyOf(e)))

if (newErrors.length === 0) {
	console.log(
		`[typecheck] OK\uff1a\u65e0\u65b0\u589e\u7c7b\u578b\u9519\u8bef\uff08\u5f53\u524d ${errors.length} \u6761\uff0c\u57fa\u7ebf ${baseline.size} \u6761\uff09`,
	)
	process.exit(0)
}

console.error(`[typecheck] \u53d1\u73b0 ${newErrors.length} \u6761\u65b0\u589e\u7c7b\u578b\u9519\u8bef\uff1a`)
const byFile = new Map()
for (const e of newErrors) {
	if (!byFile.has(e.file)) byFile.set(e.file, [])
	byFile.get(e.file).push(e)
}
for (const [file, list] of byFile) {
	const tag = baselineFiles.has(file) ? '' : ' (\u65b0\u6587\u4ef6)'
	console.error(`\n  ${file}${tag}`)
	for (const e of list.slice(0, 10)) {
		console.error(`    ${e.line}:${e.col} ${e.code}: ${e.message.slice(0, 160)}`)
	}
	if (list.length > 10) console.error(`    \u2026\u5171 ${list.length} \u6761`)
}
console.error(
	'\n\u5982\u679c\u786e\u5b9a\u8fd9\u4e9b\u9519\u8bef\u4e0d\u5f71\u54cd\u8fd0\u884c\uff0c\u53ef\u6267\u884c node scripts/typecheck-regression.mjs --update \u66f4\u65b0\u57fa\u7ebf',
)
process.exit(1)
