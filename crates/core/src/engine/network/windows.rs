//! Read the current user's enabled WinINet proxy without enabling WPAD or changing settings.
use std::ffi::c_void;

#[repr(C)]
struct IeProxyConfig {
    auto_detect: i32,
    auto_config_url: *mut u16,
    proxy: *mut u16,
    proxy_bypass: *mut u16,
}
#[link(name = "winhttp")]
unsafe extern "system" {
    fn WinHttpGetIEProxyConfigForCurrentUser(config: *mut IeProxyConfig) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalFree(memory: *mut c_void) -> *mut c_void;
}
pub(super) fn static_proxy() -> Result<Option<String>, ()> {
    let mut config = IeProxyConfig {
        auto_detect: 0,
        auto_config_url: std::ptr::null_mut(),
        proxy: std::ptr::null_mut(),
        proxy_bypass: std::ptr::null_mut(),
    };
    let ok = unsafe { WinHttpGetIEProxyConfigForCurrentUser(&mut config) } != 0;
    let pac = ok && !config.auto_config_url.is_null() && unsafe { *config.auto_config_url != 0 };
    let proxy = if ok && !config.proxy.is_null() {
        let mut length = 0;
        // Windows owns a NUL-terminated UTF-16 allocation, freed below even on failure.
        unsafe {
            while *config.proxy.add(length) != 0 {
                length += 1;
            }
            Some(String::from_utf16_lossy(std::slice::from_raw_parts(
                config.proxy,
                length,
            )))
        }
    } else {
        None
    };
    for pointer in [config.auto_config_url, config.proxy, config.proxy_bypass] {
        if !pointer.is_null() {
            unsafe {
                GlobalFree(pointer.cast());
            }
        }
    }
    if proxy.is_none() && pac {
        Err(())
    } else {
        Ok(proxy)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn native_current_user_proxy_abi_does_not_crash_or_expose_values() {
        // Exercise allocation/free and the real WinHTTP ABI on native Windows.
        let _ = super::static_proxy();
    }
}
