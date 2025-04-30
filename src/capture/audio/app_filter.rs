#[cfg(target_os = "windows")]
use windows::{
    core::HSTRING,
    Win32::Media::Audio::{IAudioSessionControl2, IAudioSessionManager},
    Win32::System::ProcessStatus::GetModuleFileNameExW,
    Win32::System::Threading::{OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION},
};

#[cfg(target_os = "windows")]
pub fn is_kook_process(pid: u32) -> bool {
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid)
            .expect("Failed to open process");
        let mut filename = [0u16; 260];
        let len = GetModuleFileNameExW(process, None, &mut filename);
        if len == 0 {
            return false;
        }
        let exe_name = HSTRING::from_wide(&filename[..len as usize])
            .expect("Failed to convert filename");
        exe_name.to_string_lossy().contains("Kook.exe")
    }
}

#[cfg(not(target_os = "windows"))]
pub fn is_kook_process(_pid: u32) -> bool {
    false
}
