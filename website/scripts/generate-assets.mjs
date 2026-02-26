#!/usr/bin/env node
/**
 * Generate PNG favicons, apple-touch-icon, and OG image from SVG sources.
 * Uses sharp (already a project dependency).
 *
 * Usage: node scripts/generate-assets.mjs
 */
import { readFileSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { fileURLToPath } from 'node:url';
import sharp from 'sharp';

const __dirname = dirname(fileURLToPath(import.meta.url));
const publicDir = resolve(__dirname, '../public');

const faviconSvg = readFileSync(resolve(publicDir, 'favicon.svg'));

// --- Favicon PNGs ---
async function generateFavicons() {
  const sizes = [
    { name: 'favicon-16x16.png', size: 16 },
    { name: 'favicon-32x32.png', size: 32 },
    { name: 'apple-touch-icon.png', size: 180 },
  ];

  for (const { name, size } of sizes) {
    await sharp(faviconSvg)
      .resize(size, size)
      .png()
      .toFile(resolve(publicDir, name));
    console.log(`Generated ${name} (${size}x${size})`);
  }
}

// --- OG Image (1200x630) ---
async function generateOgImage() {
  // Read the logo SVG (no background version) for embedding
  const logoSvg = readFileSync(resolve(publicDir, 'logo.svg'), 'utf8');

  // Extract the inner <g> content from logo.svg for embedding
  const gMatch = logoSvg.match(/<g[^>]*>([\s\S]*?)<\/g>/);
  const logoInner = gMatch ? gMatch[0] : '';

  const ogSvg = `<svg width="1200" height="630" xmlns="http://www.w3.org/2000/svg">
  <defs>
    <linearGradient id="bg" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0%" stop-color="#0f1729"/>
      <stop offset="100%" stop-color="#1e293b"/>
    </linearGradient>
  </defs>

  <!-- Background -->
  <rect width="1200" height="630" fill="url(#bg)"/>

  <!-- Subtle grid pattern -->
  <g opacity="0.03" stroke="#d97757" stroke-width="1">
    ${Array.from({ length: 25 }, (_, i) => `<line x1="${i * 50}" y1="0" x2="${i * 50}" y2="630"/>`).join('\n    ')}
    ${Array.from({ length: 13 }, (_, i) => `<line x1="0" y1="${i * 50}" x2="1200" y2="${i * 50}"/>`).join('\n    ')}
  </g>

  <!-- Logo starburst (scaled up, centered) -->
  <g transform="translate(600,250) scale(3.5)">
    ${logoInner.replace('transform="translate(64,64)"', '')}
  </g>

  <!-- Title -->
  <text x="600" y="460" text-anchor="middle"
        font-family="system-ui, -apple-system, 'Segoe UI', sans-serif"
        font-size="80" font-weight="bold" fill="white"
        letter-spacing="4">Claudex</text>

  <!-- URL -->
  <text x="600" y="530" text-anchor="middle"
        font-family="system-ui, -apple-system, 'Segoe UI', sans-serif"
        font-size="22" fill="#d97757">claudex.space</text>

  <!-- Bottom accent line -->
  <rect x="0" y="615" width="1200" height="15" fill="#d97757" opacity="0.8" rx="0"/>

  <!-- Corner accents -->
  <rect x="40" y="40" width="60" height="3" fill="#d97757" opacity="0.3" rx="1.5"/>
  <rect x="40" y="40" width="3" height="60" fill="#d97757" opacity="0.3" rx="1.5"/>
  <rect x="1100" y="40" width="60" height="3" fill="#d97757" opacity="0.3" rx="1.5"/>
  <rect x="1157" y="40" width="3" height="60" fill="#d97757" opacity="0.3" rx="1.5"/>
</svg>`;

  await sharp(Buffer.from(ogSvg))
    .png()
    .toFile(resolve(publicDir, 'og.png'));
  console.log('Generated og.png (1200x630)');
}

// --- Run ---
async function main() {
  await generateFavicons();
  await generateOgImage();
  console.log('All assets generated.');
}

main().catch((err) => {
  console.error('Asset generation failed:', err);
  process.exit(1);
});
