const fs = require('node:fs'), path = require('node:path'), {execFileSync} = require('node:child_process');

const UNSIGNED_BODY = (version) => `Aero ${version} Windows x64 portable, built by GitHub Actions.\n\nThis build carries no Aero release signature, so it is not offered through the automatic update feed. Aero keeps updating from the signed latest release. Download this portable to test this exact build.\n\nTo publish a signed release for this version, delete this release first, then sign it on the publisher computer with npm run release:publish.`;

async function publishUnsigned({token, repository, artifact, version, commit}) {
  const headers = {Authorization:`Bearer ${token}`,Accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28','User-Agent':'Aero-Release-Publisher'};
  async function api(endpoint, method='GET', body) {
    const response = await fetch('https://api.github.com'+endpoint, {method,headers:{...headers,'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});
    if (response.status===404 && method==='GET') return null;
    if (!response.ok) throw Error(`GitHub release request failed (${response.status}).`);
    return response.status===204 ? null : response.json();
  }
  const prefix = `/repos/${repository}`;
  let release = await api(`${prefix}/releases/tags/v${version}`);
  if (release && !release.draft) {
    console.log(`Release v${version} is already published; leaving it untouched.`);
    return null;
  }
  if (!release) release = await api(`${prefix}/releases`,'POST',{tag_name:`v${version}`,target_commitish:commit,name:`Aero ${version}`,draft:true,prerelease:false,body:UNSIGNED_BODY(version)});
  const bytes = fs.readFileSync(artifact);
  const name = path.basename(artifact);
  const old = release.assets.find(a=>a.name===name);
  if (old) await api(`${prefix}/releases/assets/${old.id}`,'DELETE');
  const upload = release.upload_url.replace(/\{.*$/,'')+'?name='+encodeURIComponent(name);
  const response = await fetch(upload,{method:'POST',headers:{...headers,'Content-Type':'application/octet-stream','Content-Length':String(bytes.length)},body:bytes});
  if (!response.ok) throw Error(`Release asset upload failed (${response.status}).`);
  const asset = await response.json();
  if (asset.size!==bytes.length) throw Error('Uploaded release size did not match.');
  await api(`${prefix}/releases/${release.id}`,'PATCH',{draft:false,make_latest:'false'});
  return `https://github.com/${repository}/releases/tag/v${version}`;
}

async function publishPortable() {
  const version = JSON.parse(fs.readFileSync('package.json','utf8')).version;
  const configuration = JSON.parse(fs.readFileSync('src-tauri/updater.json','utf8'));
  const repository = configuration.repository;
  const artifact = `portable/Aero-Rust-Portable-${version}-x64.exe`;
  if (!fs.existsSync(artifact)) throw Error('The portable executable is missing.');
  const token = process.env.GITHUB_TOKEN;
  if (!token) throw Error('A GitHub release credential is required.');
  const commit = process.env.GITHUB_SHA || execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  const privateKey = process.env.AERO_UPDATE_PRIVATE_KEY;
  if (privateKey && privateKey.trim()) {
    const {prepare} = require('./sign-release.cjs');
    const {publish} = require('./publish-release.cjs');
    prepare({artifact,output:'release',privateKey,passphrase:process.env.AERO_UPDATE_PRIVATE_KEY_PASSWORD,configuration,version});
    return publish({token,repository,directory:'release',artifact,version,commit});
  }
  return publishUnsigned({token,repository,artifact,version,commit});
}

module.exports = {publishPortable, publishUnsigned};

if (require.main === module) {
  publishPortable().then(url => { if (url) console.log(`Published ${url}`); })
    .catch(cause => { console.error(cause.message); process.exitCode = 1; });
}
