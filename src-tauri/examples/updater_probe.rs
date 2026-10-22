// Integration fixture only. This example is never bundled into Aero releases.
#![allow(dead_code)]
#[path="../src/update/core.rs"]
mod core;
use std::{fs,path::PathBuf};
fn main(){if !cfg!(debug_assertions){std::process::exit(2);}if let Err(e)=run(){eprintln!("{e}");std::process::exit(1);}}
fn run()->Result<(),String>{
    let args=std::env::args().skip(1).collect::<Vec<_>>();
    if let Ok(original)=std::env::var("AERO_LAUNCHER_PATH"){
        let root=PathBuf::from(original).parent().unwrap().to_path_buf();
        let version="1.0.5";
        if let Ok(nonce)=std::env::var("AERO_UPDATE_HEALTH"){core::atomic(&root.join(".aero-updates").join(format!("health-{nonce}")),version.as_bytes())?;}
        fs::write(root.join("last-started-version"),version).map_err(|_|"Cannot report fixture version")?;
        return Ok(());
    }
    let config:core::Config=serde_json::from_slice(&fs::read(&args[1]).map_err(|_|"No fixture config")?).map_err(|_|"Invalid fixture config")?;
    let root=std::env::current_exe().map_err(|_|"No fixture path")?.parent().unwrap().to_path_buf();
    let store=core::Store::new(&root,config)?;
    match args[0].as_str(){
        "stage"=>{
            let record:core::Record=serde_json::from_slice(&fs::read(&args[2]).map_err(|_|"No fixture record")?).map_err(|_|"Invalid fixture record")?;
            let payload=fs::File::open(&args[3]).map_err(|_|"No fixture payload")?;
            let _lock=store.lock()?;
            let installed=store.stage(&record,&args[4],payload)?;println!("{installed}");
        }
        "run"=>{
            let original=std::env::current_exe().map_err(|_|"No fixture launcher")?;
            if !store.launch_cached(&args[2],&original,&[]){fs::write(root.join("last-started-version"),&args[2]).map_err(|_|"Cannot report fallback")?;}
        }
        "lock"=>{let _lock=store.lock()?;fs::write(root.join("locked"),"yes").map_err(|_|"Cannot report lock")?;std::thread::sleep(std::time::Duration::from_secs(3));}
        _=>return Err("Unknown fixture operation".into()),
    }
    Ok(())
}
