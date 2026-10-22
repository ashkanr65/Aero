const {test} = require('node:test');
const assert = require('node:assert/strict');
const fs = require('node:fs'), os = require('node:os'), path = require('node:path');
const {publishUnsigned} = require('./publish-portable.cjs');

function jsonResponse(status, body) {
  return {status, ok: status >= 200 && status < 300, json: async () => body};
}

function withFetch(handler, run) {
  const original = globalThis.fetch;
  globalThis.fetch = handler;
  return Promise.resolve(run()).finally(() => { globalThis.fetch = original; });
}

function syntheticArtifact(root, version) {
  const artifact = path.join(root, `Aero-Rust-Portable-${version}-x64.exe`);
  const bytes = Buffer.alloc(1024);
  bytes.write('MZ');
  fs.writeFileSync(artifact, bytes);
  return artifact;
}

test('an unsigned portable is published without claiming the latest release', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'aero-publish-synthetic-'));
  try {
    const version = '1.0.5';
    const artifact = syntheticArtifact(root, version);
    const calls = [];
    await withFetch(async (url, options) => {
      calls.push({url: String(url), method: options.method, body: options.body});
      if (String(url).includes('/releases/tags/')) return jsonResponse(404, {});
      if (options.method === 'POST' && String(url).endsWith('/releases')) return jsonResponse(201, {id: 7, draft: true, assets: [], upload_url: 'https://uploads.github.com/repos/synthetic/Aero/releases/7/assets{?name,label}'});
      if (String(url).includes('/assets?name=')) return jsonResponse(201, {size: fs.statSync(artifact).size});
      if (options.method === 'PATCH') return jsonResponse(200, {});
      throw Error(`Unexpected request ${options.method} ${url}`);
    }, async () => {
      const url = await publishUnsigned({token: 'test-token', repository: 'synthetic/Aero', artifact, version, commit: 'abc123'});
      assert.equal(url, 'https://github.com/synthetic/Aero/releases/tag/v1.0.5');
    });
    const created = calls.find(call => call.method === 'POST' && call.url.endsWith('/releases'));
    assert.ok(created, 'the release was created');
    assert.equal(JSON.parse(created.body).draft, true);
    assert.equal(JSON.parse(created.body).tag_name, 'v1.0.5');
    assert.equal(JSON.parse(created.body).target_commitish, 'abc123');
    const upload = calls.find(call => call.url.includes('/assets?name='));
    assert.ok(upload, 'the portable asset was uploaded');
    assert.equal(decodeURIComponent(upload.url.split('name=')[1]), `Aero-Rust-Portable-${version}-x64.exe`);
    const patch = calls.find(call => call.method === 'PATCH');
    assert.ok(patch, 'the release was published');
    assert.equal(JSON.parse(patch.body).draft, false);
    assert.equal(JSON.parse(patch.body).make_latest, 'false');
  } finally { fs.rmSync(root, {recursive:true, force:true}); }
});

test('a release that is already published is left untouched', async () => {
  const root = fs.mkdtempSync(path.join(os.tmpdir(), 'aero-publish-synthetic-'));
  try {
    const version = '1.0.5';
    const artifact = syntheticArtifact(root, version);
    const calls = [];
    await withFetch(async (url, options) => {
      calls.push({url: String(url), method: options.method});
      if (String(url).includes('/releases/tags/')) return jsonResponse(200, {id: 7, draft: false, assets: []});
      throw Error(`Unexpected request ${options.method} ${url}`);
    }, async () => {
      const url = await publishUnsigned({token: 'test-token', repository: 'synthetic/Aero', artifact, version, commit: 'abc123'});
      assert.equal(url, null);
    });
    assert.equal(calls.length, 1, 'only the release lookup was made');
  } finally { fs.rmSync(root, {recursive:true, force:true}); }
});
