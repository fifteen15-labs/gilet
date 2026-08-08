// Rewrites the `url` fields in a tauri-action-generated updater manifest
// (latest.json) to point at the R2 mirror under cdn.trove.ink/releases/gilet,
// the same bucket Trove's own release pipeline uses, under a gilet-specific
// prefix. Ported from trove's scripts/ci/rewrite-updater-manifest.mjs
// (#3101's fix there): even on a public repo, GitHub release asset URLs are
// slower and less stable than a CDN mirror, and every installed app polls
// the CDN, never github.com.
//
// The artifact basename is never regex-extracted from the whole URL. It
// comes from, in order of preference:
//   1. the minisign signature's trusted comment (`file:<basename>`) — the
//      name of the exact file that was signed, present in every entry; and
//   2. the URL's last path segment, when it is a real filename (GitHub asset
//      API URLs end in an opaque numeric id and carry no filename at all).
// When both are available they must agree — a half-consistent manifest is
// never published.
//
// `signature`, `version`, `notes`, and `pub_date` travel unchanged.
//
// Usage: node scripts/ci/rewrite-updater-manifest.mjs <latest.json> <release-tag> [out-path]
// Rewrites <latest.json> in place unless [out-path] is given. Exits non-zero
// on any schema violation, underivable filename, or leftover github.com
// reference, so a broken manifest fails the release instead of shipping.

import { readFile, writeFile } from 'node:fs/promises'
import { pathToFileURL } from 'node:url'

export const CDN_RELEASE_BASE = 'https://cdn.trove.ink/releases/gilet'

// Plain vN.N.N tags only — gilet is a single package, no package-prefixed
// tags the way trove's monorepo needs.
const TAG_PATTERN = /^v\d+\.\d+\.\d+$/

const isPlainObject = (value) =>
  value !== null && typeof value === 'object' && !Array.isArray(value)

// Tauri signatures are base64 of a minisign signature file whose trusted
// comment names the signed file: `trusted comment: timestamp:<ts>\tfile:<name>`.
export function fileNameFromSignature(signature) {
  const decoded = Buffer.from(signature, 'base64').toString('utf8')
  const match = decoded.match(/^trusted comment:.*\bfile:([^\r\n]+)$/m)
  const name = match?.[1].trim()
  return name ? name : null
}

// Last path segment of the URL, percent-decoded — or null when the URL does
// not carry a filename (GitHub asset API URLs end in a numeric asset id).
export function fileNameFromUrl(rawUrl) {
  let url
  try {
    url = new URL(rawUrl)
  } catch {
    throw new Error(`not a valid URL: ${rawUrl}`)
  }
  const last = url.pathname.split('/').filter(Boolean).at(-1)
  if (!last) return null
  let decoded
  try {
    decoded = decodeURIComponent(last)
  } catch {
    throw new Error(`URL has malformed percent-encoding: ${rawUrl}`)
  }
  if (/^\d+$/.test(decoded)) return null
  const basename = decoded.split('/').at(-1)
  return basename ? basename : null
}

export function rewriteUpdaterManifest(manifest, releaseTag) {
  if (typeof releaseTag !== 'string' || !TAG_PATTERN.test(releaseTag)) {
    throw new Error(`release tag ${JSON.stringify(releaseTag)} is not a plain vN.N.N tag`)
  }
  if (!isPlainObject(manifest)) {
    throw new Error('manifest is not a JSON object')
  }
  const { version, platforms } = manifest
  if (typeof version !== 'string' || version.length === 0) {
    throw new Error("manifest has no 'version' string")
  }
  const tagVersion = releaseTag.slice(1)
  if (tagVersion !== version) {
    throw new Error(`manifest version ${version} does not match release tag ${releaseTag}`)
  }
  if (!isPlainObject(platforms) || Object.keys(platforms).length === 0) {
    throw new Error("manifest has no platform entries under 'platforms'")
  }

  const rewrittenPlatforms = {}
  const mappings = []
  for (const [platform, entry] of Object.entries(platforms)) {
    if (!isPlainObject(entry)) {
      throw new Error(`platform ${platform} is not an object`)
    }
    if (typeof entry.url !== 'string' || entry.url.length === 0) {
      throw new Error(`platform ${platform} has no 'url' string`)
    }
    if (typeof entry.signature !== 'string' || entry.signature.length === 0) {
      throw new Error(`platform ${platform} has no 'signature' string`)
    }

    const sigName = fileNameFromSignature(entry.signature)
    const urlName = fileNameFromUrl(entry.url)
    if (sigName && urlName && sigName !== urlName) {
      throw new Error(
        `platform ${platform}: signature names ${sigName} but url names ${urlName} — refusing to guess`,
      )
    }
    const basename = sigName ?? urlName
    if (!basename) {
      throw new Error(
        `platform ${platform}: cannot derive an artifact filename from the url (${entry.url}) or the signature's trusted comment`,
      )
    }

    const rewritten = {
      ...entry,
      url: `${CDN_RELEASE_BASE}/${releaseTag}/${encodeURIComponent(basename)}`,
    }
    if (JSON.stringify(rewritten).includes('github.com')) {
      throw new Error(`platform ${platform} still references github.com after rewrite`)
    }
    rewrittenPlatforms[platform] = rewritten
    mappings.push({ platform, basename, url: rewritten.url })
  }

  return { manifest: { ...manifest, platforms: rewrittenPlatforms }, mappings }
}

async function main() {
  const [manifestPath, releaseTag, outPath] = process.argv.slice(2)
  if (!manifestPath || !releaseTag) {
    throw new Error('Usage: rewrite-updater-manifest.mjs <latest.json> <release-tag> [out-path]')
  }

  const raw = await readFile(manifestPath, 'utf8')
  let parsed
  try {
    parsed = JSON.parse(raw)
  } catch (error) {
    throw new Error(`${manifestPath} is not valid JSON: ${error.message}`, { cause: error })
  }

  const { manifest, mappings } = rewriteUpdaterManifest(parsed, releaseTag)
  await writeFile(outPath ?? manifestPath, `${JSON.stringify(manifest, null, 2)}\n`)

  for (const { platform, url } of mappings) {
    console.log(`${platform} -> ${url}`)
  }
  console.log(`Rewrote ${mappings.length} platform url(s) for ${releaseTag}`)
}

if (import.meta.url === pathToFileURL(process.argv[1] ?? '').href) {
  try {
    await main()
  } catch (error) {
    console.error(
      `::error::rewrite-updater-manifest: ${error instanceof Error ? error.message : error}`,
    )
    process.exitCode = 1
  }
}
