//! Ключ сессии claude.ai хранится только на этом компьютере и зашифрован DPAPI:
//! расшифровать файл может лишь тот же пользователь Windows на той же машине.

use std::path::{Path, PathBuf};

fn file(dir: &Path) -> PathBuf {
    dir.join("session.key")
}

#[cfg(windows)]
fn crypt(data: &[u8], encrypt: bool) -> Option<Vec<u8>> {
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{
        CryptProtectData, CryptUnprotectData, CRYPT_INTEGER_BLOB,
    };
    unsafe {
        let inp = CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 };
        let mut out = CRYPT_INTEGER_BLOB::default();
        let r = if encrypt {
            CryptProtectData(&inp, PCWSTR::null(), None, None, None, 0, &mut out)
        } else {
            CryptUnprotectData(&inp, None, None, None, None, 0, &mut out)
        };
        r.ok()?;
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(out.pbData as _));
        Some(v)
    }
}

#[cfg(not(windows))]
fn crypt(_data: &[u8], _encrypt: bool) -> Option<Vec<u8>> {
    None
}

pub fn save(dir: &Path, key: &str) -> Result<(), String> {
    let key = key.trim();
    if key.is_empty() {
        let _ = std::fs::remove_file(file(dir));
        return Ok(());
    }
    let enc = crypt(key.as_bytes(), true).ok_or("не удалось зашифровать ключ")?;
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    std::fs::write(file(dir), enc).map_err(|e| e.to_string())
}

pub fn load(dir: &Path) -> Option<String> {
    let enc = std::fs::read(file(dir)).ok()?;
    String::from_utf8(crypt(&enc, false)?).ok().filter(|s| !s.is_empty())
}
