use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, Payload},
};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};

use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::{SystemTime, UNIX_EPOCH},
};
use url::Url;
use uuid::Uuid;
use zeroize::{Zeroize, ZeroizeOnDrop, Zeroizing};

const AAD: &[u8] = b"Aero.PasswordVault.v1";
const MAX_VAULT_BYTES: u64 = 16 * 1024 * 1024;
static AUTH_EPOCHS: OnceLock<Mutex<HashMap<isize, u64>>> = OnceLock::new();
static VAULT_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

#[derive(Serialize, Deserialize, Clone)]
struct VaultFile {
    version: u32,
    wrapped_key: String,
    nonce: String,
    ciphertext: String,
}

#[derive(Serialize, Deserialize, Clone, Zeroize, ZeroizeOnDrop)]
struct Credential {
    id: String,
    origin: String,
    username: String,
    password: String,
    created_at: u64,
    updated_at: u64,
}

#[derive(Serialize, Deserialize, Default, Zeroize, ZeroizeOnDrop)]
struct VaultPayload {
    credentials: Vec<Credential>,
}

#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
struct CredentialInput {
    #[serde(default)]
    id: Option<String>,
    origin: String,
    username: String,
    password: String,
}
#[derive(Deserialize, Zeroize, ZeroizeOnDrop)]
struct ImportInput {
    entries: Vec<CredentialInput>,
}

fn auth_epoch(hwnd: isize) -> Result<u64, String> {
    let epochs = AUTH_EPOCHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .map_err(|_| "Authorization is unavailable".to_string())?;
    Ok(*epochs.get(&hwnd).unwrap_or(&0))
}
fn check_authorization(hwnd: isize, epoch: u64) -> Result<(), String> {
    if auth_epoch(hwnd)? != epoch {
        return Err("The vault was locked during authentication".into());
    }
    Ok(())
}

fn upsert_credential(
    payload: &mut VaultPayload,
    value: &CredentialInput,
) -> Result<String, String> {
    let origin = validate_origin(&value.origin)?;
    if value.username.is_empty()
        || value.username.len() > 512
        || value.password.is_empty()
        || value.password.len() > 8192
    {
        return Err("Username or password is empty or too long".into());
    }
    let existing = match value.id.as_ref() {
        Some(id) => Some(
            payload
                .credentials
                .iter()
                .find(|entry| entry.id == *id)
                .ok_or_else(|| "Selected credential no longer exists".to_string())?,
        ),
        None => payload
            .credentials
            .iter()
            .find(|entry| entry.origin == origin && entry.username == value.username),
    };
    let id = existing
        .map(|entry| entry.id.clone())
        .unwrap_or_else(|| Uuid::new_v4().to_string());
    let timestamp = now();
    let created_at = existing.map(|entry| entry.created_at).unwrap_or(timestamp);
    if existing.is_none() && payload.credentials.len() >= 5000 {
        return Err("The vault supports up to 5,000 credentials".into());
    }
    payload.credentials.retain(|entry| entry.id != id);
    payload.credentials.push(Credential {
        id: id.clone(),
        origin,
        username: value.username.clone(),
        password: value.password.clone(),
        created_at,
        updated_at: timestamp,
    });
    Ok(id)
}

#[derive(Serialize)]
struct CredentialSummary {
    id: String,
    origin: String,
    username: String,
    updated_at: u64,
}

fn vault_path() -> Result<PathBuf, String> {
    let base = std::env::var_os("APPDATA")
        .ok_or_else(|| "Windows user profile storage is unavailable".to_string())?;
    Ok(PathBuf::from(base).join("Aero").join("password-vault.json"))
}

fn read_vault(path: &Path) -> Result<Option<VaultFile>, String> {
    match fs::File::open(path) {
        Ok(file) => {
            let metadata = file
                .metadata()
                .map_err(|_| "Vault could not be read safely".to_string())?;
            if metadata.len() > MAX_VAULT_BYTES {
                return Err("Vault is larger than the supported size".into());
            }
            let mut contents = Vec::with_capacity(metadata.len() as usize);
            file.take(MAX_VAULT_BYTES + 1)
                .read_to_end(&mut contents)
                .map_err(|_| "Vault could not be read safely".to_string())?;
            if contents.len() as u64 > MAX_VAULT_BYTES {
                return Err("Vault is larger than the supported size".into());
            }
            serde_json::from_slice(&contents)
                .map(Some)
                .map_err(|_| "Vault is corrupted; the original file was preserved".into())
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(_) => Err("Vault could not be opened; no data was changed".into()),
    }
}

fn validate_origin(raw: &str) -> Result<String, String> {
    let url = Url::parse(raw).map_err(|_| "Credential origin is invalid".to_string())?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err("Password entries require a secure HTTPS website origin".into());
    }
    if url.path() != "/" || url.query().is_some() || url.fragment().is_some() {
        return Err("Credential origin must not contain a path, query, or fragment".into());
    }
    Ok(url.origin().ascii_serialization())
}

fn summaries(payload: &VaultPayload) -> Result<String, String> {
    let public: Vec<CredentialSummary> = payload
        .credentials
        .iter()
        .map(|credential| CredentialSummary {
            id: credential.id.clone(),
            origin: credential.origin.clone(),
            username: credential.username.clone(),
            updated_at: credential.updated_at,
        })
        .collect();
    serde_json::to_string(&public).map_err(|_| "Credential list could not be encoded".into())
}

fn open_payload(
    file: Option<VaultFile>,
) -> Result<(VaultPayload, Option<Zeroizing<Vec<u8>>>, Option<VaultFile>), String> {
    match file {
        Some(file) => {
            let (payload, key) = decrypt_payload(&file)?;
            Ok((payload, Some(key), Some(file)))
        }
        None => Ok((VaultPayload::default(), None, None)),
    }
}

fn seal_payload(
    payload: &VaultPayload,
    current: Option<&VaultFile>,
    existing_key: Option<Zeroizing<Vec<u8>>>,
) -> Result<VaultFile, String> {
    let mut key = match existing_key {
        Some(key) => key,
        None => {
            let mut random = [0u8; 32];
            getrandom::fill(&mut random)
                .map_err(|_| "Secure random generation failed".to_string())?;
            let key = Zeroizing::new(random.to_vec());
            random.zeroize();
            key
        }
    };
    if key.len() != 32 {
        key.zeroize();
        return Err("Vault key is invalid; existing data was not changed".into());
    }
    let result = (|| {
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| "Vault encryption could not be initialized".to_string())?;
        let mut nonce = [0u8; 12];
        loop {
            getrandom::fill(&mut nonce)
                .map_err(|_| "Secure random generation failed".to_string())?;
            if current
                .and_then(|vault| BASE64.decode(&vault.nonce).ok())
                .as_deref()
                != Some(nonce.as_slice())
            {
                break;
            }
        }
        let mut plaintext = Zeroizing::new(
            serde_json::to_vec(payload)
                .map_err(|_| "Vault data could not be serialized".to_string())?,
        );
        let nonce = Nonce::from(nonce);
        let encrypted = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: &plaintext,
                    aad: AAD,
                },
            )
            .map_err(|_| "Vault encryption failed; existing data was not changed".to_string());
        plaintext.zeroize();
        let ciphertext = encrypted?;
        let wrapped_key = win::protect(&key)?;
        Ok(VaultFile {
            version: 1,
            wrapped_key: BASE64.encode(wrapped_key),
            nonce: BASE64.encode(nonce),
            ciphertext: BASE64.encode(ciphertext),
        })
    })();
    key.zeroize();
    result
}

fn decrypt_payload(file: &VaultFile) -> Result<(VaultPayload, Zeroizing<Vec<u8>>), String> {
    if file.version != 1 {
        return Err("Vault version is not supported; existing data was preserved".into());
    }
    let wrapped_key = BASE64
        .decode(&file.wrapped_key)
        .map_err(|_| "Vault key data is corrupted".to_string())?;
    let mut key = Zeroizing::new(win::unprotect(&wrapped_key)?);
    if key.len() != 32 {
        key.zeroize();
        return Err("Vault key is invalid".into());
    }
    let nonce_bytes: [u8; 12] = BASE64
        .decode(&file.nonce)
        .map_err(|_| "Vault data is corrupted".to_string())?
        .try_into()
        .map_err(|_| "Vault data is corrupted".to_string())?;
    let nonce = Nonce::from(nonce_bytes);
    let ciphertext = BASE64
        .decode(&file.ciphertext)
        .map_err(|_| "Vault data is corrupted".to_string())?;
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|_| "Vault encryption could not be initialized".to_string())?;
    let mut plaintext = Zeroizing::new(
        cipher
            .decrypt(
                &nonce,
                Payload {
                    msg: &ciphertext,
                    aad: AAD,
                },
            )
            .map_err(|_| "Vault authentication failed; data may have been altered".to_string())?,
    );
    let payload = match serde_json::from_slice(&plaintext) {
        Ok(payload) => payload,
        Err(_) => {
            plaintext.zeroize();
            key.zeroize();
            return Err("Vault contents are corrupted".into());
        }
    };
    plaintext.zeroize();
    Ok((payload, key))
}

#[cfg(test)]
fn write_atomic(path: &Path, bytes: &[u8]) -> Result<(), String> {
    write_authorized(path, bytes, None)
}

fn write_authorized(
    path: &Path,
    bytes: &[u8],
    authorization: Option<(isize, u64)>,
) -> Result<(), String> {
    if bytes.len() as u64 > MAX_VAULT_BYTES {
        return Err("Vault exceeds the supported size; existing data was preserved".into());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "Vault location is invalid".to_string())?;
    fs::create_dir_all(parent).map_err(|_| "Vault directory could not be created".to_string())?;
    let temporary = parent.join(format!(".vault-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&temporary)
            .map_err(|_| "Temporary vault file could not be created".to_string())?;
        file.write_all(bytes)
            .and_then(|_| file.sync_all())
            .map_err(|_| {
                "Vault write was interrupted; the original vault was preserved".to_string()
            })?;
        drop(file);
        // Keep cancellation and commit mutually exclusive. A cancelled Hello cannot commit a write.
        let epochs = AUTH_EPOCHS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .map_err(|_| "Authorization is unavailable".to_string())?;
        if let Some((hwnd, epoch)) = authorization {
            if *epochs.get(&hwnd).unwrap_or(&0) != epoch {
                return Err("The vault was locked; the original file was preserved".into());
            }
        }
        replace_file(&temporary, path).map_err(|_| {
            "Vault update could not be committed; the original vault was preserved".to_string()
        })
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result
}

#[cfg(windows)]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(Some(0))
        .collect();
    unsafe {
        MoveFileExW(
            windows::core::PCWSTR(source.as_ptr()),
            windows::core::PCWSTR(destination.as_ptr()),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    }
    .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error))
}

#[cfg(not(windows))]
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    fs::rename(source, destination)
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn perform_operation(
    operation: &str,
    input: &str,
    hwnd: isize,
    epoch: u64,
) -> Result<String, String> {
    let _guard = VAULT_LOCK
        .get_or_init(|| Mutex::new(()))
        .lock()
        .map_err(|_| "Vault is temporarily unavailable".to_string())?;
    if operation == "availability" {
        return Ok(serde_json::json!({ "available": win::hello_available()? }).to_string());
    }
    let purpose = match operation {
        "list" => "Unlock Aero Password Manager",
        "save" => "Save or update a password in Aero",
        "delete" => "Delete a password from Aero",
        "reveal" => "Reveal a saved password",
        "backup" => "Create an encrypted password vault backup",
        "restore" => "Restore an encrypted password vault backup",
        "import" => "Import passwords into Aero",
        _ => return Err("Password-manager operation is invalid".into()),
    };
    check_authorization(hwnd, epoch)?;
    win::authenticate(hwnd, purpose)?;
    check_authorization(hwnd, epoch)?;
    let path = vault_path()?;
    perform_authenticated_operation(operation, input, &path, hwnd, epoch)
}

// Private backend: production reaches this only after window-bound Windows Hello and cancellation checks.
fn perform_authenticated_operation(
    operation: &str,
    input: &str,
    path: &Path,
    hwnd: isize,
    epoch: u64,
) -> Result<String, String> {
    match operation {
        "list" => {
            let (mut payload, mut key, _) = open_payload(read_vault(&path)?)?;
            if let Some(key) = key.as_mut() {
                key.zeroize();
            }
            let result = summaries(&payload);
            for credential in &mut payload.credentials {
                credential.password.zeroize();
            }
            result
        }
        "save" => {
            let value: CredentialInput = serde_json::from_str(input)
                .map_err(|_| "Credential data is invalid".to_string())?;
            let (mut payload, mut key, current) = open_payload(read_vault(&path)?)?;
            let id = upsert_credential(&mut payload, &value)?;
            let sealed = seal_payload(&payload, current.as_ref(), key.take())?;
            let serialized = serde_json::to_vec(&sealed)
                .map_err(|_| "Vault could not be serialized".to_string())?;
            write_authorized(&path, &serialized, Some((hwnd, epoch)))?;
            for credential in &mut payload.credentials {
                credential.password.zeroize();
            }
            Ok(serde_json::json!({ "id": id }).to_string())
        }
        "import" => {
            let value: ImportInput = serde_json::from_str(input)
                .map_err(|_| "Password import data is invalid".to_string())?;
            if value.entries.is_empty() || value.entries.len() > 5000 {
                return Err("Choose a CSV with between 1 and 5,000 valid passwords".into());
            }
            let (mut payload, mut key, current) = open_payload(read_vault(&path)?)?;
            let mut imported = 0usize;
            for entry in &value.entries {
                upsert_credential(&mut payload, entry)?;
                imported += 1;
            }
            let sealed = seal_payload(&payload, current.as_ref(), key.take())?;
            let serialized = serde_json::to_vec(&sealed)
                .map_err(|_| "Vault could not be serialized".to_string())?;
            write_authorized(&path, &serialized, Some((hwnd, epoch)))?;
            for credential in &mut payload.credentials {
                credential.password.zeroize();
            }
            Ok(serde_json::json!({ "imported": imported }).to_string())
        }
        "delete" => {
            let value: serde_json::Value = serde_json::from_str(input)
                .map_err(|_| "Credential selection is invalid".to_string())?;
            let id = value
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Credential selection is required".to_string())?;
            let (mut payload, mut key, current) = open_payload(read_vault(&path)?)?;
            let count = payload.credentials.len();
            payload.credentials.retain(|credential| credential.id != id);
            if payload.credentials.len() == count {
                if let Some(key) = key.as_mut() {
                    key.zeroize();
                }
                return Err("Selected credential no longer exists".into());
            }
            let sealed = seal_payload(&payload, current.as_ref(), key.take())?;
            write_authorized(
                &path,
                &serde_json::to_vec(&sealed)
                    .map_err(|_| "Vault could not be serialized".to_string())?,
                Some((hwnd, epoch)),
            )?;
            Ok("{}".into())
        }
        "reveal" => {
            let value: serde_json::Value = serde_json::from_str(input)
                .map_err(|_| "Credential selection is invalid".to_string())?;
            let id = value
                .get("id")
                .and_then(|v| v.as_str())
                .ok_or_else(|| "Credential selection is required".to_string())?;
            let file = read_vault(&path)?
                .ok_or_else(|| "Password vault has no saved credentials".to_string())?;
            let (mut payload, mut key) = decrypt_payload(&file)?;
            let credential = payload
                .credentials
                .iter()
                .position(|credential| credential.id == id)
                .map(|index| payload.credentials.remove(index))
                .ok_or_else(|| "Selected credential no longer exists".to_string());
            key.zeroize();
            for entry in &mut payload.credentials {
                entry.password.zeroize();
            }
            let mut credential = credential?;
            let result = serde_json::to_string(&credential)
                .map_err(|_| "Credential could not be encoded".to_string());
            credential.password.zeroize();
            result
        }
        "backup" => {
            let vault =
                read_vault(&path)?.ok_or_else(|| "There is no vault to back up yet".to_string())?;
            Ok(BASE64.encode(
                serde_json::to_vec(&vault)
                    .map_err(|_| "Encrypted backup could not be encoded".to_string())?,
            ))
        }
        "restore" => {
            let encoded: String = serde_json::from_str(input)
                .map_err(|_| "Encrypted backup input is invalid".to_string())?;
            if encoded.len() > (MAX_VAULT_BYTES as usize * 2) {
                return Err("Encrypted backup exceeds the supported size".into());
            }
            let bytes = BASE64
                .decode(&encoded)
                .map_err(|_| "Encrypted backup is invalid".to_string())?;
            let backup: VaultFile = serde_json::from_slice(&bytes)
                .map_err(|_| "Encrypted backup is corrupted".to_string())?;
            let (payload, mut key) = decrypt_payload(&backup)?;
            key.zeroize();
            drop(payload);
            write_authorized(&path, &bytes, Some((hwnd, epoch)))?;
            Ok("{}".into())
        }
        _ => Err("Password-manager operation is invalid".into()),
    }
}

#[cfg(windows)]
mod win {
    use super::*;
    use windows::{
        Security::Credentials::UI::{
            UserConsentVerificationResult, UserConsentVerifier, UserConsentVerifierAvailability,
        },
        Win32::{
            Foundation::{HLOCAL, HWND, LocalFree},
            Security::Cryptography::{
                CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
            },
            System::WinRT::{
                IUserConsentVerifierInterop, RO_INIT_MULTITHREADED, RoInitialize, RoUninitialize,
            },
        },
        core::{HSTRING, factory},
    };

    struct WinRtApartment;
    impl WinRtApartment {
        fn init() -> Result<Self, String> {
            unsafe { RoInitialize(RO_INIT_MULTITHREADED) }
                .map(|_| Self)
                .map_err(|_| "Windows authentication could not initialize its native API".into())
        }
    }
    impl Drop for WinRtApartment {
        fn drop(&mut self) {
            unsafe { RoUninitialize() };
        }
    }

    pub fn hello_available() -> Result<bool, String> {
        let _apartment = WinRtApartment::init()?;
        let availability = UserConsentVerifier::CheckAvailabilityAsync()
            .map_err(|_| "Windows Hello availability could not be checked".to_string())?
            .join()
            .map_err(|_| "Windows Hello availability could not be checked".to_string())?;
        Ok(availability == UserConsentVerifierAvailability::Available)
    }

    pub fn authenticate(hwnd: isize, purpose: &str) -> Result<(), String> {
        let _apartment = WinRtApartment::init()?;
        if hwnd == 0 {
            return Err("Aero could not attach Windows authentication to its window".into());
        }
        let interop: IUserConsentVerifierInterop = factory::<UserConsentVerifier, IUserConsentVerifierInterop>()
            .map_err(|_| "Windows Hello is unavailable. Enable it in Windows Settings > Accounts > Sign-in options.".to_string())?;
        let operation = unsafe { interop.RequestVerificationForWindowAsync::<windows_future::IAsyncOperation<UserConsentVerificationResult>>(HWND(hwnd as *mut _), &HSTRING::from(purpose)) }
            .map_err(|_| "Windows Hello could not start. Enable it in Windows Settings > Accounts > Sign-in options.".to_string())?;
        let result = operation
            .join()
            .map_err(|_| "Windows authentication failed".to_string())?;
        if result == UserConsentVerificationResult::Verified {
            Ok(())
        } else {
            Err(match result {
            UserConsentVerificationResult::Canceled => "Windows authentication was cancelled".into(),
            UserConsentVerificationResult::DeviceNotPresent | UserConsentVerificationResult::NotConfiguredForUser => "Windows Hello is unavailable. Configure it in Windows Settings > Accounts > Sign-in options.".into(),
            UserConsentVerificationResult::DisabledByPolicy => "Windows Hello is disabled by your organization".into(),
            UserConsentVerificationResult::RetriesExhausted => "Windows authentication was denied after too many failed attempts".into(),
            _ => "Windows authentication was not successful".into(),
        })
        }
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe {
            CryptProtectData(
                &input,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut output,
            )
        }
        .map_err(|_| "Windows current-user encryption failed".to_string())?;
        let protected =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe { LocalFree(Some(HLOCAL(output.pbData.cast()))) };
        Ok(protected)
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        let input = CRYPT_INTEGER_BLOB {
            cbData: data.len() as u32,
            pbData: data.as_ptr() as *mut u8,
        };
        let mut output = CRYPT_INTEGER_BLOB::default();
        unsafe { CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output) }
            .map_err(|_| "The vault key is unavailable to this Windows account. The vault file was left untouched.".to_string())?;
        let plaintext =
            unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
        unsafe { std::slice::from_raw_parts_mut(output.pbData, output.cbData as usize) }.zeroize();
        unsafe { LocalFree(Some(HLOCAL(output.pbData.cast()))) };
        Ok(plaintext)
    }
}

#[cfg(not(windows))]
mod win {
    pub fn hello_available() -> Result<bool, String> {
        Err("Password Manager requires Windows Hello on Windows".into())
    }
    pub fn authenticate(_: isize, _: &str) -> Result<(), String> {
        Err("Windows Hello is only available on Windows".into())
    }
    pub fn protect(_: &[u8]) -> Result<Vec<u8>, String> {
        Err("Current-user Windows encryption is unavailable".into())
    }
    pub fn unprotect(_: &[u8]) -> Result<Vec<u8>, String> {
        Err("Current-user Windows encryption is unavailable".into())
    }
}

pub fn availability() -> Result<bool, String> {
    win::hello_available()
}
pub fn operation(op: &str, input: &str, hwnd: isize, epoch: u64) -> Result<String, String> {
    perform_operation(op, input, hwnd, epoch)
}
pub fn epoch(hwnd: isize) -> Result<u64, String> {
    auth_epoch(hwnd)
}
pub fn cancel(hwnd: isize) {
    if let Ok(mut values) = AUTH_EPOCHS
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
    {
        let value = values.entry(hwnd).or_default();
        *value = value.wrapping_add(1);
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aes_gcm_rejects_ciphertext_tampering() {
        let key = [7u8; 32];
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let nonce = [9u8; 12];
        let nonce = Nonce::from(nonce);
        let encrypted = cipher
            .encrypt(
                &nonce,
                Payload {
                    msg: b"synthetic password",
                    aad: AAD,
                },
            )
            .unwrap();
        assert_eq!(
            cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: &encrypted,
                        aad: AAD
                    }
                )
                .unwrap(),
            b"synthetic password"
        );
        let mut changed = encrypted;
        let last = changed.len() - 1;
        changed[last] ^= 1;
        assert!(
            cipher
                .decrypt(
                    &nonce,
                    Payload {
                        msg: &changed,
                        aad: AAD
                    }
                )
                .is_err()
        );
    }

    #[test]
    fn origin_validation_rejects_http_and_paths() {
        assert_eq!(
            validate_origin("https://example.test/").unwrap(),
            "https://example.test"
        );
        assert!(validate_origin("http://example.test/").is_err());
        assert!(validate_origin("https://example.test/login").is_err());
        assert!(validate_origin("https://name:pass@example.test/").is_err());
    }

    #[test]
    fn malformed_vault_is_not_treated_as_empty() {
        let dir = std::env::temp_dir().join(format!("aero-vault-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.json");
        fs::write(&path, b"broken").unwrap();
        assert!(read_vault(&path).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"broken");
        fs::remove_dir_all(dir).unwrap();
    }
    fn synthetic() -> CredentialInput {
        CredentialInput {
            id: None,
            origin: "https://example.test".into(),
            username: "synthetic-user".into(),
            password: "synthetic-only-password".into(),
        }
    }

    #[test]
    fn save_updates_matching_account_without_duplicate() {
        let mut payload = VaultPayload::default();
        let mut entry = synthetic();
        let first = upsert_credential(&mut payload, &entry).unwrap();
        entry.password = "synthetic-updated-password".into();
        assert_eq!(upsert_credential(&mut payload, &entry).unwrap(), first);
        assert_eq!(payload.credentials.len(), 1);
        assert_eq!(payload.credentials[0].password, entry.password);
        entry.id = Some("missing-id".into());
        assert!(upsert_credential(&mut payload, &entry).is_err());
        assert_eq!(payload.credentials.len(), 1);
    }

    #[test]
    #[cfg(windows)]
    fn dpapi_roundtrip_restart_backup_restore_and_tampering() {
        let dir = std::env::temp_dir().join(format!("aero-synthetic-vault-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.json");
        let mut payload = VaultPayload::default();
        let id = upsert_credential(&mut payload, &synthetic()).unwrap();
        let file = seal_payload(&payload, None, None).unwrap();
        let bytes = serde_json::to_vec(&file).unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("synthetic-only-password"));
        write_atomic(&path, &bytes).unwrap();
        let read = read_vault(&path).unwrap().unwrap();
        let (restored, _) = decrypt_payload(&read).unwrap();
        assert_eq!(restored.credentials[0].id, id);
        assert_eq!(restored.credentials[0].password, "synthetic-only-password");
        assert!(
            !summaries(&restored)
                .unwrap()
                .contains("synthetic-only-password")
        );
        let backup = BASE64.encode(&bytes);
        let mut changed_payload = VaultPayload::default();
        let mut entry = synthetic();
        entry.password = "changed-for-restore".into();
        upsert_credential(&mut changed_payload, &entry).unwrap();
        let changed_file = seal_payload(&changed_payload, Some(&read), None).unwrap();
        assert_ne!(changed_file.nonce, read.nonce);
        write_atomic(&path, &serde_json::to_vec(&changed_file).unwrap()).unwrap();
        let backup_bytes = BASE64.decode(backup).unwrap();
        let backup_file: VaultFile = serde_json::from_slice(&backup_bytes).unwrap();
        decrypt_payload(&backup_file).unwrap();
        write_atomic(&path, &backup_bytes).unwrap();
        assert_eq!(
            decrypt_payload(&read_vault(&path).unwrap().unwrap())
                .unwrap()
                .0
                .credentials[0]
                .password,
            "synthetic-only-password"
        );
        let mut tampered = read.clone();
        let mut ciphertext = BASE64.decode(&tampered.ciphertext).unwrap();
        ciphertext[0] ^= 1;
        tampered.ciphertext = BASE64.encode(ciphertext);
        assert!(decrypt_payload(&tampered).is_err());
        tampered = read.clone();
        tampered.wrapped_key = BASE64.encode(b"inaccessible-windows-key");
        assert!(decrypt_payload(&tampered).is_err());
        assert_eq!(fs::read(&path).unwrap(), bytes);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn cancellation_preserves_original_and_cleans_temporary_write() {
        let dir = std::env::temp_dir().join(format!("aero-cancel-vault-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.json");
        fs::write(&path, b"original-encrypted-data").unwrap();
        let hwnd = 987654321isize;
        AUTH_EPOCHS
            .get_or_init(|| Mutex::new(HashMap::new()))
            .lock()
            .unwrap()
            .insert(hwnd, 2);
        assert!(check_authorization(hwnd, 1).is_err());
        assert!(write_authorized(&path, b"replacement", Some((hwnd, 1))).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"original-encrypted-data");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        write_authorized(&path, b"replacement", Some((hwnd, 2))).unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"replacement");
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn failed_atomic_replace_preserves_target() {
        let dir = std::env::temp_dir().join(format!("aero-interrupted-vault-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let target = dir.join("unreplaceable-directory");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("original"), b"preserved").unwrap();
        assert!(write_atomic(&target, b"replacement").is_err());
        assert_eq!(fs::read(target.join("original")).unwrap(), b"preserved");
        assert_eq!(fs::read_dir(&dir).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    #[cfg(windows)]
    fn authenticated_backend_save_update_delete_and_restore_json_protocol() {
        let dir = std::env::temp_dir().join(format!("aero-backend-test-{}", Uuid::new_v4()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("vault.json");
        let hwnd = 876543210isize;
        let epoch = auth_epoch(hwnd).unwrap();
        let entry = serde_json::json!({"origin":"https://example.test","username":"synthetic-user","password":"synthetic-password"}).to_string();
        let first: serde_json::Value = serde_json::from_str(
            &perform_authenticated_operation("save", &entry, &path, hwnd, epoch).unwrap(),
        )
        .unwrap();
        let id = first["id"].as_str().unwrap();
        let list: Vec<serde_json::Value> = serde_json::from_str(
            &perform_authenticated_operation("list", "{}", &path, hwnd, epoch).unwrap(),
        )
        .unwrap();
        assert_eq!(list.len(), 1);
        assert!(list[0].get("password").is_none());
        let selection = serde_json::json!({"id":id}).to_string();
        let revealed: serde_json::Value = serde_json::from_str(
            &perform_authenticated_operation("reveal", &selection, &path, hwnd, epoch).unwrap(),
        )
        .unwrap();
        assert_eq!(revealed["password"], "synthetic-password");
        let backup = perform_authenticated_operation("backup", "{}", &path, hwnd, epoch).unwrap();
        let update = serde_json::json!({"origin":"https://example.test","username":"synthetic-user","password":"synthetic-updated"}).to_string();
        let updated: serde_json::Value = serde_json::from_str(
            &perform_authenticated_operation("save", &update, &path, hwnd, epoch).unwrap(),
        )
        .unwrap();
        assert_eq!(updated["id"], id);
        perform_authenticated_operation("delete", &selection, &path, hwnd, epoch).unwrap();
        assert_eq!(
            perform_authenticated_operation("list", "{}", &path, hwnd, epoch).unwrap(),
            "[]"
        );
        // Main-process nativeOperation JSON-encodes its payload, including backup strings.
        let restore_input = serde_json::to_string(&backup).unwrap();
        perform_authenticated_operation("restore", &restore_input, &path, hwnd, epoch).unwrap();
        let restored: serde_json::Value = serde_json::from_str(
            &perform_authenticated_operation("reveal", &selection, &path, hwnd, epoch).unwrap(),
        )
        .unwrap();
        assert_eq!(restored["password"], "synthetic-password");
        let original = fs::read(&path).unwrap();
        assert!(
            perform_authenticated_operation("restore", "\"invalid\"", &path, hwnd, epoch).is_err()
        );
        assert_eq!(fs::read(&path).unwrap(), original);
        fs::write(&path, b"corrupted").unwrap();
        assert!(perform_authenticated_operation("save", &entry, &path, hwnd, epoch).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"corrupted");
        fs::remove_dir_all(dir).unwrap();
    }
}
