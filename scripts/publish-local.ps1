$ErrorActionPreference='Stop'
Push-Location (Split-Path -Parent $PSScriptRoot)
try {
    if(git status --porcelain){throw 'Commit the release source before publishing.'}
    $version=(Get-Content package.json -Raw|ConvertFrom-Json).version
    $tags=@(git tag --points-at HEAD)
    if($tags -notcontains "v$version"){throw "Tag the current commit v$version first."}
    npm test
    if($LASTEXITCODE){throw 'Release verification failed'}
    npm run package:portable
    if($LASTEXITCODE){throw 'Portable build failed'}
    Add-Type -AssemblyName System.Security.Cryptography.ProtectedData
    $keyPath=Join-Path $env:LOCALAPPDATA 'AeroPublisher\update-signing.dpapi'
    $clear=[Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes($keyPath),$null,[Security.Cryptography.DataProtectionScope]::CurrentUser)
    $key=[Text.Encoding]::UTF8.GetString($clear)|ConvertFrom-Json
    $env:AERO_UPDATE_PRIVATE_KEY=$key.privateKey
    $env:AERO_UPDATE_PRIVATE_KEY_PASSWORD=$key.passphrase
    node scripts/sign-release.cjs
    if($LASTEXITCODE){throw 'Release signing failed'}
    # Only the public executable, manifest and signatures reach GitHub.
    Remove-Item Env:AERO_UPDATE_PRIVATE_KEY,Env:AERO_UPDATE_PRIVATE_KEY_PASSWORD -ErrorAction SilentlyContinue
    [Array]::Clear($clear);$key=$null
    $env:GIT_TERMINAL_PROMPT='0'
    $env:GCM_INTERACTIVE='never'
    $credential="protocol=https`nhost=github.com`n`n"|git credential fill
    if($LASTEXITCODE){throw 'GitHub authentication is unavailable'}
    $passwordLine=$credential|Where-Object {$_ -like 'password=*'}|Select-Object -First 1
    if(-not $passwordLine){throw 'GitHub authentication is unavailable'}
    $env:GITHUB_TOKEN=$passwordLine.Substring(9)
    node scripts/publish-release.cjs
    if($LASTEXITCODE){throw 'Release publishing failed'}
} finally {
    Remove-Item Env:AERO_UPDATE_PRIVATE_KEY,Env:AERO_UPDATE_PRIVATE_KEY_PASSWORD,Env:GITHUB_TOKEN -ErrorAction SilentlyContinue
    if($clear){[Array]::Clear($clear)}
    $key=$null;$credential=$null;$passwordLine=$null
    Pop-Location
}
