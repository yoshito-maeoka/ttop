#[cfg(target_os = "macos")]
mod inner {
    use std::ffi::c_void;
    use std::mem;
    use std::sync::OnceLock;

    type ResponsibilityFn = unsafe extern "C" fn(i32) -> i32;
    type ProcPidInfoFn = unsafe extern "C" fn(i32, i32, u64, *mut c_void, i32) -> i32;

    const PROC_PIDT_SHORTBSDINFO: i32 = 13;

    #[repr(C)]
    struct ProcBsdShortInfo {
        pbsi_pid: u32,
        pbsi_ppid: u32,
        pbsi_pgid: u32,
        pbsi_status: u32,
        pbsi_comm: [u8; 16],
        pbsi_flags: u32,
        pbsi_uid: u32,
        pbsi_gid: u32,
        pbsi_ruid: u32,
        pbsi_rgid: u32,
        pbsi_svuid: u32,
        pbsi_svgid: u32,
        pbsi_rfu: u32,
    }

    static RESPONSIBILITY_FN: OnceLock<Option<ResponsibilityFn>> = OnceLock::new();
    static PROC_PIDINFO_FN: OnceLock<Option<ProcPidInfoFn>> = OnceLock::new();

    fn get_responsibility_fn() -> Option<ResponsibilityFn> {
        *RESPONSIBILITY_FN.get_or_init(|| unsafe {
            let handle = libc::dlopen(std::ptr::null(), libc::RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
            let symbol_name = b"responsibility_get_pid_responsible_for_pid\0";
            let symbol = libc::dlsym(handle, symbol_name.as_ptr() as *const i8);
            if symbol.is_null() {
                None
            } else {
                Some(std::mem::transmute::<*mut c_void, ResponsibilityFn>(symbol))
            }
        })
    }

    fn get_proc_pidinfo_fn() -> Option<ProcPidInfoFn> {
        *PROC_PIDINFO_FN.get_or_init(|| unsafe {
            let handle = libc::dlopen(std::ptr::null(), libc::RTLD_LAZY);
            if handle.is_null() {
                return None;
            }
            let symbol_name = b"proc_pidinfo\0";
            let symbol = libc::dlsym(handle, symbol_name.as_ptr() as *const i8);
            if symbol.is_null() {
                None
            } else {
                Some(std::mem::transmute::<*mut c_void, ProcPidInfoFn>(symbol))
            }
        })
    }

    pub fn get_process_group(pid: u32) -> Option<u32> {
        let func = get_proc_pidinfo_fn()?;
        unsafe {
            let mut info: ProcBsdShortInfo = mem::zeroed();
            let size = mem::size_of::<ProcBsdShortInfo>() as i32;

            let ret = func(
                pid as i32,
                PROC_PIDT_SHORTBSDINFO,
                0,
                &mut info as *mut _ as *mut c_void,
                size,
            );

            if ret > 0 {
                Some(info.pbsi_pgid)
            } else {
                None
            }
        }
    }

    pub fn get_responsible_pid(pid: u32) -> Option<u32> {
        if let Some(func) = get_responsibility_fn() {
            let responsible = unsafe { func(pid as i32) };
            if responsible > 0 && responsible != pid as i32 {
                return Some(responsible as u32);
            }
        }
        None
    }

    pub fn get_parent_app_name(process_name: &str) -> Option<&'static str> {
        let name_lower = process_name.to_lowercase();

        if name_lower.contains("webkit") && name_lower.starts_with("com.apple.webkit") {
            return Some("Safari");
        }

        if name_lower.contains("google chrome helper") {
            return Some("Google Chrome");
        }

        if name_lower.contains("firefox") && name_lower.contains("helper") {
            return Some("Firefox");
        }

        if name_lower.contains("slack") && name_lower.contains("helper") {
            return Some("Slack");
        }

        if name_lower.contains("code helper") || name_lower.contains("electron helper") {
            if name_lower.contains("code") {
                return Some("Code");
            }
        }

        None
    }
}

#[cfg(not(target_os = "macos"))]
mod inner {
    pub fn get_responsible_pid(_pid: u32) -> Option<u32> {
        None
    }

    pub fn get_process_group(_pid: u32) -> Option<u32> {
        None
    }

    pub fn get_parent_app_name(_process_name: &str) -> Option<&'static str> {
        None
    }
}

pub use inner::*;
