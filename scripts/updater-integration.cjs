const fs=require('node:fs'),path=require('node:path'),os=require('node:os'),crypto=require('node:crypto'),assert=require('node:assert/strict'),{spawnSync,spawn}=require('node:child_process');
const {setTimeout:delay}=require('node:timers/promises');
async function run(){
  const root=fs.mkdtempSync(path.join(os.tmpdir(),'aero-updater-launch-'));
  const dir=path.join(root,'folder with spaces');fs.mkdirSync(dir);
  const source=path.resolve(process.env.CARGO_TARGET_DIR||'target','debug/examples/updater_probe.exe');
  const launcher=path.join(dir,'Aero-Portable-Test.exe');fs.copyFileSync(source,launcher);
  const payload=fs.readFileSync(source);
  const keys=crypto.generateKeyPairSync('ed25519');
  const config=path.join(root,'config.json');fs.writeFileSync(config,JSON.stringify({repository:'synthetic/Aero',publicKey:Buffer.from(keys.publicKey.export({format:'jwk'}).x,'base64url').toString('base64'),channel:'stable'}));
  function signed(version,bytes){const metadata={schema:1,version,target:'windows-x86_64',url:`https://github.com/synthetic/Aero/releases/download/v${version}/Aero-Rust-Portable-${version}-x64.exe`,size:bytes.length,sha256:crypto.createHash('sha256').update(bytes).digest('hex')};const manifest=JSON.stringify(metadata);return {manifest,signature:crypto.sign(null,Buffer.from(manifest),keys.privateKey).toString('base64')};}
  function recordFile(version,bytes){const p=path.join(root,version+'.json');fs.writeFileSync(p,JSON.stringify(signed(version,bytes)));return p;}
  function exec(args,ok=true){const result=spawnSync(launcher,args,{encoding:'utf8',windowsHide:true,timeout:15000});assert.equal(result.status===0,ok,result.stderr||result.error?.message);return result;}
  const checks=[];
  try{
    fs.mkdirSync(path.join(dir,'AeroData'));fs.writeFileSync(path.join(dir,'AeroData','history-fixture'),'synthetic existing data');
    exec(['run',config,'1.0.4']);assert.equal(fs.readFileSync(path.join(dir,'last-started-version'),'utf8'),'1.0.4');checks.push('offline launch without updates');
    const good=recordFile('1.0.5',payload),truncated=path.join(root,'truncated.exe');fs.writeFileSync(truncated,payload.subarray(0,400));
    exec(['stage',config,good,truncated,'1.0.4'],false);assert(!fs.existsSync(path.join(dir,'.aero-updates','active.json')));checks.push('interrupted download keeps original');
    exec(['stage',config,good,source,'1.0.4']);assert.equal(fs.readFileSync(path.join(dir,'last-started-version'),'utf8'),'1.0.4');checks.push('background staging preserves running version');
    exec(['run',config,'1.0.4']);assert.equal(fs.readFileSync(path.join(dir,'last-started-version'),'utf8'),'1.0.5');checks.push('next start automatically runs signed new executable');
    const bad=recordFile('1.0.6',payload);exec(['stage',config,bad,source,'1.0.5']);
    // Probe acknowledges its compiled 1.0.5; a mismatched 1.0.6 release fails health and falls back.
    exec(['run',config,'1.0.4']);assert.equal(fs.readFileSync(path.join(dir,'last-started-version'),'utf8'),'1.0.5');assert.equal(fs.readFileSync(path.join(dir,'.aero-updates','failed-version'),'utf8'),'1.0.6');checks.push('failed startup falls back to previous healthy version');
    const tampered=JSON.parse(fs.readFileSync(good,'utf8'));tampered.manifest+=' ';fs.writeFileSync(path.join(root,'tampered.json'),JSON.stringify(tampered));exec(['stage',config,path.join(root,'tampered.json'),source,'1.0.4'],false);checks.push('tampered metadata is never activated');
    const lock=spawn(launcher,['lock',config],{windowsHide:true,stdio:'ignore'});
    for(let i=0;i<50&&!fs.existsSync(path.join(dir,'locked'));i++)await delay(50);
    exec(['stage',config,recordFile('1.0.7',payload),source,'1.0.5'],false);await new Promise(resolve=>lock.once('exit',resolve));checks.push('concurrent processes cannot change an update together');
    assert.equal(fs.readFileSync(path.join(dir,'AeroData','history-fixture'),'utf8'),'synthetic existing data');checks.push('portable data remains in original location');
    console.log(JSON.stringify({passed:true,checks},null,2));
  }finally{fs.rmSync(root,{recursive:true,force:true});}
}
run().catch(e=>{console.error(e);process.exitCode=1;});
