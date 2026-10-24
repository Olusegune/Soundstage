//! Encryption at rest for consent records, using the Windows Data Protection API.
//!
//! A protected file can be opened only by the same Windows user on the same computer. There is no key for Soundstage to
//! lose or leak: Windows holds it. Copying the `.consent` folder to another computer or user account leaves the records
//! unreadable, which is the point.

#[cfg(windows)]
mod imp {
    use windows_sys::Win32::Foundation::LocalFree;
    use windows_sys::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    const ENTROPY: &[u8] = b"Soundstage consent record v1";

    fn blob(b: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: b.len() as u32, pbData: b.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let v = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        LocalFree(out.pbData as _);
        v
    }

    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() > u32::MAX as usize {
            return Err("That record is too large to protect.".into());
        }
        let (input, entropy) = (blob(data), blob(ENTROPY));
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: the blobs point at live slices for the duration of the call, and the output is copied then freed.
        unsafe {
            if CryptProtectData(&input, std::ptr::null(), &entropy, std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
                return Err("Windows could not protect the record.".into());
            }
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        if data.len() > u32::MAX as usize {
            return Err("That record is too large to open.".into());
        }
        let (input, entropy) = (blob(data), blob(ENTROPY));
        let mut out = CRYPT_INTEGER_BLOB { cbData: 0, pbData: std::ptr::null_mut() };
        // SAFETY: as above.
        unsafe {
            if CryptUnprotectData(&input, std::ptr::null_mut(), &entropy, std::ptr::null(), std::ptr::null(), CRYPTPROTECT_UI_FORBIDDEN, &mut out) == 0 {
                return Err("This record can only be opened by the Windows user and computer that made it.".into());
            }
            Ok(take(out))
        }
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn protect(data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
    pub fn unprotect(data: &[u8]) -> Result<Vec<u8>, String> {
        Ok(data.to_vec())
    }
}

const MAGIC: &[u8] = b"SSENC1\n";

/// Protect `data` for storage. The result starts with a marker so old, unprotected files can still be recognised.
pub fn seal(data: &[u8]) -> Result<Vec<u8>, String> {
    let mut out = MAGIC.to_vec();
    out.extend(imp::protect(data)?);
    Ok(out)
}

/// Open a stored file: protected files are decrypted, and older plain files are returned as they are.
/// The flag says whether the file was already protected.
pub fn open(stored: &[u8]) -> Result<(Vec<u8>, bool), String> {
    match stored.strip_prefix(MAGIC) {
        Some(rest) => Ok((imp::unprotect(rest)?, true)),
        None => Ok((stored.to_vec(), false)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_hides_the_text() {
        let secret = b"{\"voice_subject\":\"Ama Okafor\"}";
        let sealed = seal(secret).unwrap();
        assert!(sealed.starts_with(MAGIC));
        if cfg!(windows) {
            assert!(!sealed.windows(secret.len()).any(|w| w == &secret[..]), "the plain text must not appear in the file");
            assert!(!String::from_utf8_lossy(&sealed).contains("Okafor"));
        }
        assert_eq!(open(&sealed).unwrap(), (secret.to_vec(), true));
    }

    #[test]
    fn plain_files_from_older_versions_are_still_readable() {
        assert_eq!(open(b"{\"a\":1}").unwrap(), (b"{\"a\":1}".to_vec(), false));
    }

    #[test]
    fn damaged_protected_files_are_refused() {
        if !cfg!(windows) {
            return;
        }
        let mut sealed = seal(b"hello").unwrap();
        let last = sealed.len() - 1;
        sealed[last] ^= 0xff;
        sealed[MAGIC.len() + 20] ^= 0x55;
        assert!(open(&sealed).is_err());
        assert!(open(&[MAGIC, b"short"].concat()).is_err());
    }
}
