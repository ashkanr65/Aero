const fs = require('node:fs'), path = require('node:path'), crypto = require('node:crypto'), {execFileSync} = require('node:child_process');

async function publish({token, repository, directory = 'release', artifact, version, commit}) {
  if (!token) throw Error('A GitHub release credential is required.');
  const config = JSON.parse(fs.readFileSync('src-tauri/updater.json','utf8'));
  if (repository !== config.repository) throw Error('The release repository must match Aero’s update configuration.');
  const manifest = fs.readFileSync(path.join(directory,'latest.json'));
  const metadata = JSON.parse(manifest);
  const signature = Buffer.from(fs.readFileSync(path.join(directory,'latest.json.sig'),'utf8').trim(),'base64');
  const key = crypto.createPublicKey({format:'jwk',key:{kty:'OKP',crv:'Ed25519',x:Buffer.from(config.publicKey,'base64').toString('base64url')}});
  const bytes = fs.readFileSync(artifact);
  const name = `Aero-Rust-Portable-${version}-x64.exe`;
  if (!crypto.verify(null,manifest,key,signature) || metadata.version!==version || metadata.url!==`https://github.com/${repository}/releases/download/v${version}/${name}` || metadata.size!==bytes.length || metadata.sha256!==crypto.createHash('sha256').update(bytes).digest('hex')) throw Error('Release verification failed; nothing was published.');
  const headers = {Authorization:`Bearer ${token}`,Accept:'application/vnd.github+json','X-GitHub-Api-Version':'2022-11-28','User-Agent':'Aero-Release-Publisher'};
  async function api(endpoint, method='GET', body) {
    const response=await fetch('https://api.github.com'+endpoint,{method,headers:{...headers,'Content-Type':'application/json'},body:body===undefined?undefined:JSON.stringify(body)});
    if(response.status===404 && method==='GET')return null;
    if(!response.ok)throw Error(`GitHub release request failed (${response.status}).`);
    return response.status===204?null:response.json();
  }
  const prefix=`/repos/${repository}`;
  let release=await api(`${prefix}/releases/tags/v${version}`);
  if(release && !release.draft)throw Error('This version is already public. Increase the version before publishing another update.');
  if(!release) release=await api(`${prefix}/releases`,'POST',{tag_name:`v${version}`,target_commitish:commit,name:`Aero ${version}`,draft:true,prerelease:false,body:'Signed Windows x64 portable release. Automatic updates download in the background and open on the next launch. Your browser profile stays in place.\n\nExisting 1.0.3 installations need to download this updater-enabled release once; later releases are automatic. Source-browser password exports and Windows Hello still require your approval.'});
  const files=[{name,bytes},{name:'latest.json',bytes:manifest},{name:'latest.json.sig',bytes:fs.readFileSync(path.join(directory,'latest.json.sig'))},{name:name+'.sig',bytes:fs.readFileSync(path.join(directory,name+'.sig'))}];
  for(const file of files){
    const old=release.assets.find(a=>a.name===file.name);if(old)await api(`${prefix}/releases/assets/${old.id}`,'DELETE');
    const upload=release.upload_url.replace(/\{.*$/,'')+'?name='+encodeURIComponent(file.name);
    const response=await fetch(upload,{method:'POST',headers:{...headers,'Content-Type':'application/octet-stream','Content-Length':String(file.bytes.length)},body:file.bytes});
    if(!response.ok)throw Error(`Release asset upload failed (${response.status}); release remains a draft.`);
    const asset=await response.json();if(asset.size!==file.bytes.length)throw Error('Uploaded release size did not match; release remains a draft.');
  }
  const published=await api(`${prefix}/releases/${release.id}`,'PATCH',{draft:false,make_latest:'true'});
  return published.html_url;
}
module.exports={publish};
if(require.main===module){
  const version=JSON.parse(fs.readFileSync('package.json','utf8')).version;
  const repository=JSON.parse(fs.readFileSync('src-tauri/updater.json','utf8')).repository;
  const commit=process.env.GITHUB_SHA || execFileSync('git',['rev-parse','HEAD'],{encoding:'utf8'}).trim();
  publish({token:process.env.GITHUB_TOKEN,repository,version,commit,artifact:`portable/Aero-Rust-Portable-${version}-x64.exe`}).then(url=>console.log(`Published ${url}`)).catch(cause=>{console.error(cause.message);process.exitCode=1;});
}
