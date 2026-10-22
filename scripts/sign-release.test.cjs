const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs'), os = require('node:os'), path = require('node:path'), crypto = require('node:crypto');
const {prepare} = require('./sign-release.cjs');
test('release manifest binds the version, destination, payload and trusted signing key', () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'aero-sign-synthetic-'));
  try {
    const keys = crypto.generateKeyPairSync('ed25519');
    const privateKey = keys.privateKey.export({format: 'pem', type: 'pkcs8'});
    const configuration = {repository: 'synthetic/Aero', publicKey: Buffer.from(keys.publicKey.export({format: 'jwk'}).x, 'base64url').toString('base64')};
    const artifact = path.join(root,'synthetic.exe'); const bytes = Buffer.alloc(512); bytes.write('MZ'); fs.writeFileSync(artifact, bytes);
    const args = {artifact, output: root, privateKey, configuration, version: '1.0.5'};
    const metadata = prepare(args); const manifest = fs.readFileSync(path.join(root,'latest.json'));
    const signature = Buffer.from(fs.readFileSync(path.join(root,'latest.json.sig'),'utf8').trim(),'base64');
    assert(crypto.verify(null, manifest, keys.publicKey, signature));
    assert(!crypto.verify(null, Buffer.concat([manifest, Buffer.from(' ')]), keys.publicKey, signature));
    assert.equal(metadata.sha256,crypto.createHash('sha256').update(bytes).digest('hex'));
    assert.equal(metadata.url,'https://github.com/synthetic/Aero/releases/download/v1.0.5/Aero-Rust-Portable-1.0.5-x64.exe');
    assert.throws(() => prepare({...args, version:'../unsafe'}));
    assert.throws(() => prepare({...args, configuration:{...configuration, publicKey:'wrong-key'}}));
  } finally { fs.rmSync(root,{recursive:true,force:true}); }
});
