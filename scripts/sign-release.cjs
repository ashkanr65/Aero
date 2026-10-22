const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');

function prepare({artifact, output, privateKey, passphrase, configuration, version}) {
  if (!/^\d+\.\d+\.\d+$/.test(version)) throw Error('Use a stable three-part release version.');
  const key = crypto.createPrivateKey({key: privateKey, format: 'pem', passphrase});
  const publicKey = crypto.createPublicKey(key);
  const raw = Buffer.from(publicKey.export({format: 'jwk'}).x, 'base64url').toString('base64');
  if (raw !== configuration.publicKey) throw Error('Release key does not match the verification key embedded in Aero.');
  const bytes = fs.readFileSync(artifact);
  if (bytes.length < 256 || bytes.length > 128 * 1024 * 1024 || bytes.subarray(0,2).toString() !== 'MZ') throw Error('Expected a Windows application smaller than 128 MiB.');
  const metadata = {
    schema: 1, version, target: 'windows-x86_64',
    url: `https://github.com/${configuration.repository}/releases/download/v${version}/Aero-Rust-Portable-${version}-x64.exe`,
    size: bytes.length, sha256: crypto.createHash('sha256').update(bytes).digest('hex'),
  };
  const manifest = Buffer.from(JSON.stringify(metadata, null, 2) + '\n');
  const signature = crypto.sign(null, manifest, key);
  if (!crypto.verify(null, manifest, publicKey, signature)) throw Error('Release signature self-check failed.');
  fs.mkdirSync(output, {recursive: true});
  fs.writeFileSync(path.join(output, 'latest.json'), manifest);
  fs.writeFileSync(path.join(output, 'latest.json.sig'), signature.toString('base64') + '\n');
  fs.writeFileSync(path.join(output, `Aero-Rust-Portable-${version}-x64.exe.sig`), crypto.sign(null, bytes, key).toString('base64') + '\n');
  return metadata;
}
module.exports = {prepare};
if (require.main === module) {
  try {
    const version = JSON.parse(fs.readFileSync('package.json', 'utf8')).version;
    const configuration = JSON.parse(fs.readFileSync('src-tauri/updater.json', 'utf8'));
    const metadata = prepare({artifact: process.argv[2] || `portable/Aero-Rust-Portable-${version}-x64.exe`, output: process.argv[3] || 'release', privateKey: process.env.AERO_UPDATE_PRIVATE_KEY, passphrase: process.env.AERO_UPDATE_PRIVATE_KEY_PASSWORD, configuration, version});
    console.log(`Signed Aero ${metadata.version}: ${metadata.size} bytes, SHA-256 ${metadata.sha256}`);
  } catch { console.error('Release signing failed. Check the build, protected signing key, and public-key configuration.'); process.exitCode = 1; }
}
