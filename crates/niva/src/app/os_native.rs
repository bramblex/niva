//! Small native queries shared by the Node OS API and immutable startup data.
//! Dynamic values are read on every call; this module deliberately has no cache.

use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, ToSocketAddrs};

pub(crate) fn cpus() -> Result<Value> {
    #[cfg(target_os = "macos")]
    {
        macos_cpus()
    }
    #[cfg(target_os = "linux")]
    {
        linux_cpus()
    }
    #[cfg(target_os = "windows")]
    {
        windows_cpus()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Ok(json!([]))
    }
}

pub(crate) fn free_memory() -> Result<u64> {
    #[cfg(target_os = "macos")]
    {
        macos_free_memory()
    }
    #[cfg(target_os = "linux")]
    {
        linux_memory().map(|(_, free)| free)
    }
    #[cfg(target_os = "windows")]
    {
        windows_memory().map(|(_, free)| free)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        bail!("free memory query is unsupported on this platform")
    }
}

pub(crate) fn total_memory() -> Option<u64> {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSProcessInfo;
        Some(NSProcessInfo::processInfo().physicalMemory())
    }
    #[cfg(target_os = "linux")]
    {
        linux_memory().ok().map(|(total, _)| total)
    }
    #[cfg(target_os = "windows")]
    {
        windows_memory().ok().map(|(total, _)| total)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        None
    }
}

pub(crate) fn uptime() -> Result<f64> {
    #[cfg(target_os = "macos")]
    {
        use objc2_foundation::NSProcessInfo;
        Ok(NSProcessInfo::processInfo().systemUptime())
    }
    #[cfg(target_os = "linux")]
    {
        let mut value = std::mem::MaybeUninit::<libc::timespec>::uninit();
        // SAFETY: clock_gettime initializes the provided timespec on success.
        let status = unsafe { libc::clock_gettime(libc::CLOCK_BOOTTIME, value.as_mut_ptr()) };
        if status != 0 {
            return Err(std::io::Error::last_os_error()).context("read system uptime");
        }
        // SAFETY: successful clock_gettime initialized the value.
        let value = unsafe { value.assume_init() };
        Ok(value.tv_sec as f64 + value.tv_nsec as f64 / 1_000_000_000.0)
    }
    #[cfg(target_os = "windows")]
    {
        use windows::Win32::System::SystemInformation::GetTickCount64;
        // GetTickCount64 is the Windows system uptime counter in milliseconds.
        Ok(unsafe { GetTickCount64() } as f64 / 1000.0)
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        bail!("system uptime query is unsupported on this platform")
    }
}

pub(crate) fn network_interfaces() -> Result<Value> {
    #[cfg(unix)]
    {
        unix_network_interfaces()
    }
    #[cfg(target_os = "windows")]
    {
        windows_network_interfaces()
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        Ok(json!({}))
    }
}

pub(crate) fn dns_servers() -> Result<Value> {
    #[cfg(target_os = "macos")]
    {
        macos_dns_servers()
    }
    #[cfg(target_os = "windows")]
    {
        windows_dns_servers()
    }
    #[cfg(target_os = "linux")]
    {
        linux_dns_servers()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        bail!("system DNS configuration query is unsupported on this platform")
    }
}

/// Resolve through the host operating system's resolver, including its hosts
/// file and per-interface resolver policy. No DNS server is selected here.
pub(crate) fn dns_lookup(hostname: &str, family: u8, all: bool) -> Result<Value> {
    if hostname.trim().is_empty() {
        bail!("EINVAL: hostname must be a non-empty string");
    }
    if !matches!(family, 0 | 4 | 6) {
        bail!("EINVAL: family must be 0, 4, or 6");
    }

    let resolved = (hostname, 0)
        .to_socket_addrs()
        .map_err(|error| anyhow!("ENOTFOUND: {hostname}: {error}"))?;
    let mut seen = std::collections::HashSet::new();
    let addresses: Vec<Value> = resolved
        .filter_map(|address| {
            let ip = address.ip();
            let address_family = if ip.is_ipv4() { 4 } else { 6 };
            if family != 0 && address_family != family {
                return None;
            }
            let address = ip.to_string();
            if !seen.insert((address.clone(), address_family)) {
                return None;
            }
            Some(json!({ "address": address, "family": address_family }))
        })
        .collect();

    if addresses.is_empty() {
        bail!("ENOTFOUND: no address records for {hostname}");
    }
    if all {
        Ok(json!(addresses))
    } else {
        Ok(addresses
            .into_iter()
            .next()
            .expect("non-empty checked above"))
    }
}

#[derive(Clone, Debug)]
pub(crate) struct OsVersion {
    pub(crate) os_type: String,
    pub(crate) release: String,
    pub(crate) version: String,
}

pub(crate) fn version() -> Option<OsVersion> {
    #[cfg(unix)]
    {
        unix_version()
    }
    #[cfg(target_os = "windows")]
    {
        let info = os_info::get();
        Some(OsVersion {
            os_type: "Windows_NT".to_string(),
            release: info.version().to_string(),
            version: info.to_string(),
        })
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        None
    }
}

pub(crate) fn hostname() -> Option<String> {
    #[cfg(unix)]
    {
        unix_hostname()
    }
    #[cfg(target_os = "windows")]
    {
        windows_hostname()
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        None
    }
}

pub(crate) fn user_info() -> Option<Value> {
    #[cfg(unix)]
    {
        unix_user_info()
    }
    #[cfg(target_os = "windows")]
    {
        windows_user_info()
    }
    #[cfg(not(any(unix, target_os = "windows")))]
    {
        None
    }
}

fn cpu_value(
    model: &str,
    speed: u64,
    user: u64,
    nice: u64,
    sys: u64,
    idle: u64,
    irq: u64,
) -> Value {
    json!({
        "model": model,
        "speed": speed,
        "times": { "user": user, "nice": nice, "sys": sys, "idle": idle, "irq": irq }
    })
}

#[cfg(any(target_os = "macos", target_os = "linux", test))]
fn milliseconds_from_ticks(ticks: u64, ticks_per_second: u64) -> u64 {
    ticks.saturating_mul(1000) / ticks_per_second.max(1)
}

#[cfg(target_os = "linux")]
fn linux_memory() -> Result<(u64, u64)> {
    // SAFETY: sysinfo writes one fully initialized kernel snapshot on success.
    let mut info = unsafe { std::mem::zeroed::<libc::sysinfo>() };
    // SAFETY: info is writable and has the expected libc ABI layout.
    if unsafe { libc::sysinfo(&mut info) } != 0 {
        return Err(std::io::Error::last_os_error()).context("read Linux memory status");
    }
    let multiplier = u64::from(info.mem_unit);
    Ok((
        (info.totalram as u64).saturating_mul(multiplier),
        (info.freeram as u64).saturating_mul(multiplier),
    ))
}

#[cfg(target_os = "windows")]
fn windows_memory() -> Result<(u64, u64)> {
    use windows::Win32::System::SystemInformation::{GlobalMemoryStatusEx, MEMORYSTATUSEX};
    let mut info = MEMORYSTATUSEX {
        dwLength: std::mem::size_of::<MEMORYSTATUSEX>() as u32,
        ..Default::default()
    };
    // SAFETY: the API requires dwLength to be initialized and fills the struct.
    unsafe { GlobalMemoryStatusEx(&mut info) }.context("read Windows memory status")?;
    Ok((info.ullTotalPhys, info.ullAvailPhys))
}

#[cfg(target_os = "macos")]
fn macos_free_memory() -> Result<u64> {
    #[allow(deprecated)]
    let host = unsafe { libc::mach_host_self() };
    // SAFETY: vm_statistics64 is a plain C output struct; the kernel writes it.
    let mut info = unsafe { std::mem::zeroed::<libc::vm_statistics64>() };
    let mut count = libc::HOST_VM_INFO64_COUNT;
    // SAFETY: `info` is writable and count is initialized to its ABI size.
    let status = unsafe {
        libc::host_statistics64(
            host,
            libc::HOST_VM_INFO64,
            (&mut info as *mut libc::vm_statistics64).cast(),
            &mut count,
        )
    };
    if status != libc::KERN_SUCCESS {
        bail!("read macOS memory status failed with Mach status {status}");
    }
    // SAFETY: sysconf is a read-only query; a positive result is the page size.
    let page_size = unsafe { libc::sysconf(libc::_SC_PAGESIZE) };
    if page_size <= 0 {
        bail!("read macOS page size failed");
    }
    Ok(u64::from(info.free_count).saturating_mul(page_size as u64))
}

#[cfg(target_os = "linux")]
fn linux_cpus() -> Result<Value> {
    let stat = std::fs::read_to_string("/proc/stat").context("read Linux CPU counters")?;
    let cpuinfo = std::fs::read_to_string("/proc/cpuinfo").context("read Linux CPU model")?;
    let (model, speed) = parse_linux_cpuinfo(&cpuinfo);
    // SAFETY: sysconf does not write through pointers; it returns the clock tick rate.
    let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
    let cpus: Vec<Value> = stat
        .lines()
        .filter_map(|line| parse_linux_cpu_line(line))
        .map(|times| {
            cpu_value(
                &model,
                speed,
                milliseconds_from_ticks(times[0], ticks_per_second),
                milliseconds_from_ticks(times[1], ticks_per_second),
                milliseconds_from_ticks(times[2], ticks_per_second),
                milliseconds_from_ticks(times[3].saturating_add(times[4]), ticks_per_second),
                milliseconds_from_ticks(times[5].saturating_add(times[6]), ticks_per_second),
            )
        })
        .collect();
    if cpus.is_empty() {
        bail!("Linux did not report any processor counters");
    }
    Ok(json!(cpus))
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_cpu_line(line: &str) -> Option<[u64; 7]> {
    let mut fields = line.split_whitespace();
    let label = fields.next()?;
    if label == "cpu"
        || !label.starts_with("cpu")
        || !label[3..].bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let mut values = [0; 7];
    for value in &mut values {
        *value = fields.next()?.parse().ok()?;
    }
    Some(values)
}

#[cfg(any(target_os = "linux", test))]
fn parse_linux_cpuinfo(contents: &str) -> (String, u64) {
    let mut model = None;
    let mut speed = None;
    for line in contents.lines() {
        let Some((key, value)) = line.split_once(':') else {
            continue;
        };
        match key.trim() {
            "model name" | "Hardware" | "Processor" if model.is_none() => {
                model = Some(value.trim().to_string());
            }
            "cpu MHz" if speed.is_none() => {
                speed = value.trim().parse::<f64>().ok().map(|mhz| mhz as u64);
            }
            _ => {}
        }
    }
    (
        model.unwrap_or_else(|| "Unknown processor".to_string()),
        speed.unwrap_or(0),
    )
}

#[cfg(target_os = "macos")]
fn macos_cpus() -> Result<Value> {
    #[allow(deprecated)]
    let host = unsafe { libc::mach_host_self() };
    let mut processor_count: libc::natural_t = 0;
    let mut processor_info: libc::processor_info_array_t = std::ptr::null_mut();
    let mut info_count: libc::mach_msg_type_number_t = 0;
    // SAFETY: out pointers are initialized and the returned Mach allocation is
    // released with vm_deallocate below after its contents have been copied.
    let status = unsafe {
        libc::host_processor_info(
            host,
            libc::PROCESSOR_CPU_LOAD_INFO,
            &mut processor_count,
            &mut processor_info,
            &mut info_count,
        )
    };
    if status != libc::KERN_SUCCESS || processor_info.is_null() {
        bail!("read macOS processor counters failed with Mach status {status}");
    }

    let result = (|| {
        let max_count = (info_count as usize) / libc::CPU_STATE_MAX as usize;
        let count = (processor_count as usize).min(max_count);
        // SAFETY: host_processor_info returned `info_count` integer_t values.
        let ticks = unsafe { std::slice::from_raw_parts(processor_info, info_count as usize) };
        let (model, speed) = macos_cpu_model_speed();
        // SAFETY: sysconf is read-only; positive value is the system clock rate.
        let ticks_per_second = unsafe { libc::sysconf(libc::_SC_CLK_TCK) }.max(1) as u64;
        let cpus: Vec<Value> = (0..count)
            .map(|index| {
                let values = &ticks[index * 4..index * 4 + 4];
                cpu_value(
                    &model,
                    speed,
                    milliseconds_from_ticks(
                        values[libc::CPU_STATE_USER as usize] as u64,
                        ticks_per_second,
                    ),
                    milliseconds_from_ticks(
                        values[libc::CPU_STATE_NICE as usize] as u64,
                        ticks_per_second,
                    ),
                    milliseconds_from_ticks(
                        values[libc::CPU_STATE_SYSTEM as usize] as u64,
                        ticks_per_second,
                    ),
                    milliseconds_from_ticks(
                        values[libc::CPU_STATE_IDLE as usize] as u64,
                        ticks_per_second,
                    ),
                    0,
                )
            })
            .collect();
        if cpus.is_empty() {
            bail!("macOS did not report any processor counters");
        }
        Ok(json!(cpus))
    })();

    let bytes = (info_count as libc::vm_size_t)
        .saturating_mul(std::mem::size_of::<libc::integer_t>() as libc::vm_size_t);
    #[allow(deprecated)]
    let task = unsafe { libc::mach_task_self_ as libc::vm_map_t };
    // SAFETY: this address and byte count are the allocation returned by
    // host_processor_info, and the data has already been copied above.
    let _ = unsafe { libc::vm_deallocate(task, processor_info as libc::vm_address_t, bytes) };
    result
}

#[cfg(target_os = "macos")]
fn macos_cpu_model_speed() -> (String, u64) {
    let model = sysctl_string("machdep.cpu.brand_string")
        .or_else(|| sysctl_string("hw.model"))
        .unwrap_or_else(|| "Unknown processor".to_string());
    let speed = sysctl_u64("hw.cpufrequency").unwrap_or(0) / 1_000_000;
    (model, speed)
}

#[cfg(target_os = "macos")]
fn sysctl_string(name: &str) -> Option<String> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut length = 0;
    // SAFETY: the first call only asks for the output length.
    if unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            std::ptr::null_mut(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return None;
    }
    let mut bytes = vec![0u8; length];
    // SAFETY: bytes is writable and length matches its allocation.
    if unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            bytes.as_mut_ptr().cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    } != 0
    {
        return None;
    }
    if bytes.last() == Some(&0) {
        bytes.pop();
    }
    String::from_utf8(bytes)
        .ok()
        .filter(|value| !value.is_empty())
}

#[cfg(target_os = "macos")]
fn sysctl_u64(name: &str) -> Option<u64> {
    let name = std::ffi::CString::new(name).ok()?;
    let mut value = 0u64;
    let mut length = std::mem::size_of_val(&value);
    // SAFETY: sysctlbyname writes a u64 only when the named field exists.
    let status = unsafe {
        libc::sysctlbyname(
            name.as_ptr(),
            (&mut value as *mut u64).cast(),
            &mut length,
            std::ptr::null_mut(),
            0,
        )
    };
    (status == 0 && length == std::mem::size_of_val(&value)).then_some(value)
}

#[cfg(target_os = "windows")]
fn windows_cpus() -> Result<Value> {
    use windows::Win32::System::SystemInformation::{GetSystemInfo, SYSTEM_INFO};
    let mut system_info = SYSTEM_INFO::default();
    // SAFETY: GetSystemInfo fills the initialized output structure.
    unsafe { GetSystemInfo(&mut system_info) };
    let count = system_info.dwNumberOfProcessors.max(1) as usize;
    let mut counters = vec![ProcessorPerformanceInfo::default(); count];
    let mut returned = 0u32;
    let mut byte_count = std::mem::size_of_val(counters.as_slice()) as u32;
    // GetSystemInfo can report only the current processor group. Retry once
    // with the exact size requested by the kernel on larger multi-group hosts.
    let mut status = unsafe {
        NtQuerySystemInformation(
            8, // SystemProcessorPerformanceInformation
            counters.as_mut_ptr().cast(),
            byte_count,
            &mut returned,
        )
    };
    if status as u32 == 0xC000_0004 && returned > byte_count {
        let needed = (returned as usize).div_ceil(std::mem::size_of::<ProcessorPerformanceInfo>());
        if needed <= 65_536 {
            counters.resize(needed, ProcessorPerformanceInfo::default());
            byte_count = std::mem::size_of_val(counters.as_slice()) as u32;
            returned = 0;
            // SAFETY: resized counter storage is writable and byte_count is its size.
            status = unsafe {
                NtQuerySystemInformation(8, counters.as_mut_ptr().cast(), byte_count, &mut returned)
            };
        }
    }
    if status < 0 {
        bail!(
            "read Windows processor counters failed with NTSTATUS 0x{:08x}",
            status as u32
        );
    }
    let count =
        ((returned as usize) / std::mem::size_of::<ProcessorPerformanceInfo>()).min(counters.len());
    if count == 0 {
        bail!("Windows did not report any processor counters");
    }
    let (model, speed) = windows_cpu_model_speed();
    let cpus: Vec<Value> = counters
        .into_iter()
        .take(count)
        .map(|cpu| {
            let idle = nonnegative_millis(cpu.idle_time);
            let interrupt = nonnegative_millis(cpu.interrupt_time);
            let kernel = nonnegative_millis(cpu.kernel_time);
            cpu_value(
                &model,
                speed,
                nonnegative_millis(cpu.user_time),
                0,
                kernel.saturating_sub(idle).saturating_sub(interrupt),
                idle,
                interrupt,
            )
        })
        .collect();
    Ok(json!(cpus))
}

#[cfg(target_os = "windows")]
#[repr(C)]
#[derive(Clone, Copy, Default)]
struct ProcessorPerformanceInfo {
    idle_time: i64,
    kernel_time: i64,
    user_time: i64,
    dpc_time: i64,
    interrupt_time: i64,
    interrupt_count: u32,
}

#[cfg(target_os = "windows")]
#[link(name = "ntdll")]
unsafe extern "system" {
    fn NtQuerySystemInformation(
        system_information_class: u32,
        system_information: *mut std::ffi::c_void,
        system_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[cfg(target_os = "windows")]
fn nonnegative_millis(value_100ns: i64) -> u64 {
    value_100ns.max(0) as u64 / 10_000
}

#[cfg(all(target_os = "windows", target_arch = "x86_64"))]
fn x86_cpu_model_speed() -> (String, u64) {
    let maximum = std::arch::x86_64::__cpuid(0x8000_0000).eax;
    let model = if maximum >= 0x8000_0004 {
        let mut bytes = Vec::with_capacity(48);
        for leaf in 0x8000_0002..=0x8000_0004 {
            let value = std::arch::x86_64::__cpuid(leaf);
            for word in [value.eax, value.ebx, value.ecx, value.edx] {
                bytes.extend_from_slice(&word.to_le_bytes());
            }
        }
        String::from_utf8_lossy(&bytes)
            .trim_matches('\0')
            .trim()
            .to_string()
    } else {
        String::new()
    };
    let maximum_basic = std::arch::x86_64::__cpuid(0).eax;
    let speed = if maximum_basic >= 0x16 {
        std::arch::x86_64::__cpuid(0x16).eax as u64
    } else {
        0
    };
    (
        if model.is_empty() {
            "Unknown processor".to_string()
        } else {
            model
        },
        speed,
    )
}

#[cfg(all(target_os = "windows", not(target_arch = "x86_64")))]
fn x86_cpu_model_speed() -> (String, u64) {
    ("Unknown processor".to_string(), 0)
}

#[cfg(target_os = "windows")]
fn windows_cpu_model_speed() -> (String, u64) {
    x86_cpu_model_speed()
}

#[cfg(unix)]
fn unix_version() -> Option<OsVersion> {
    // SAFETY: uname fills a properly initialized utsname structure.
    let mut info = unsafe { std::mem::zeroed::<libc::utsname>() };
    if unsafe { libc::uname(&mut info) } != 0 {
        return None;
    }
    let text = |field: &[libc::c_char]| {
        // SAFETY: each utsname field is NUL terminated by uname.
        unsafe { std::ffi::CStr::from_ptr(field.as_ptr()) }
            .to_string_lossy()
            .into_owned()
    };
    Some(OsVersion {
        os_type: text(&info.sysname),
        release: text(&info.release),
        version: text(&info.version),
    })
}

#[cfg(unix)]
fn unix_hostname() -> Option<String> {
    let mut bytes = [0 as libc::c_char; 256];
    // SAFETY: buffer is writable, sized, and nulled explicitly on success.
    if unsafe { libc::gethostname(bytes.as_mut_ptr(), bytes.len()) } != 0 {
        return None;
    }
    bytes[bytes.len() - 1] = 0;
    // SAFETY: the buffer is forced to contain a trailing NUL.
    Some(
        unsafe { std::ffi::CStr::from_ptr(bytes.as_ptr()) }
            .to_string_lossy()
            .into_owned(),
    )
}

#[cfg(unix)]
fn unix_user_info() -> Option<Value> {
    let uid = unsafe { libc::geteuid() };
    let initial = unsafe { libc::sysconf(libc::_SC_GETPW_R_SIZE_MAX) };
    let mut size = if initial > 0 {
        initial as usize
    } else {
        16 * 1024
    };
    size = size.clamp(1024, 1024 * 1024);
    loop {
        // SAFETY: passwd is an output-only C struct and is zero initialized.
        let mut entry = unsafe { std::mem::zeroed::<libc::passwd>() };
        let mut buffer = vec![0u8; size];
        let mut result = std::ptr::null_mut();
        // SAFETY: getpwuid_r receives writable storage and a matching length.
        let status = unsafe {
            libc::getpwuid_r(
                uid,
                &mut entry,
                buffer.as_mut_ptr().cast(),
                buffer.len(),
                &mut result,
            )
        };
        if status == libc::ERANGE && size < 1024 * 1024 {
            size = (size * 2).min(1024 * 1024);
            continue;
        }
        if status != 0 || result.is_null() {
            return None;
        }
        let copy = |value: *const libc::c_char| {
            if value.is_null() {
                None
            } else {
                // SAFETY: successful getpwuid_r returns NUL-terminated fields.
                Some(
                    unsafe { std::ffi::CStr::from_ptr(value) }
                        .to_string_lossy()
                        .into_owned(),
                )
            }
        };
        return Some(json!({
            "uid": entry.pw_uid,
            "gid": entry.pw_gid,
            "username": copy(entry.pw_name),
            "homedir": copy(entry.pw_dir),
            "shell": copy(entry.pw_shell),
        }));
    }
}

#[cfg(target_os = "windows")]
#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetComputerNameW(buffer: *mut u16, size: *mut u32) -> i32;
}

#[cfg(target_os = "windows")]
#[link(name = "advapi32")]
unsafe extern "system" {
    fn GetUserNameW(buffer: *mut u16, size: *mut u32) -> i32;
}

#[cfg(target_os = "windows")]
fn windows_hostname() -> Option<String> {
    let mut buffer = [0u16; 256];
    let mut length = buffer.len() as u32;
    // SAFETY: GetComputerNameW writes no more than the initialized buffer.
    if unsafe { GetComputerNameW(buffer.as_mut_ptr(), &mut length) } == 0 {
        return None;
    }
    Some(String::from_utf16_lossy(
        &buffer[..(length as usize).min(buffer.len())],
    ))
}

#[cfg(target_os = "windows")]
fn windows_user_info() -> Option<Value> {
    let mut buffer = [0u16; 256];
    let mut length = buffer.len() as u32;
    // SAFETY: GetUserNameW writes no more than the initialized buffer.
    if unsafe { GetUserNameW(buffer.as_mut_ptr(), &mut length) } == 0 {
        return None;
    }
    let username = String::from_utf16_lossy(&buffer[..(length as usize).min(buffer.len())]);
    let username = username.trim_end_matches('\0').to_string();
    Some(json!({
        "uid": -1,
        "gid": -1,
        "username": username,
        "homedir": std::env::var("USERPROFILE").ok(),
        "shell": null,
    }))
}

#[cfg(unix)]
fn unix_network_interfaces() -> Result<Value> {
    use std::collections::BTreeMap;

    // SAFETY: getifaddrs initializes the linked list; it is released once below.
    let mut head = std::ptr::null_mut();
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(std::io::Error::last_os_error()).context("read network interfaces");
    }
    let mut macs = BTreeMap::<String, String>::new();
    let mut current = head;
    while !current.is_null() {
        // SAFETY: current is a node in the getifaddrs list.
        let item = unsafe { &*current };
        let Some(name) = (if item.ifa_name.is_null() {
            None
        } else {
            // SAFETY: ifa_name is a NUL-terminated interface name.
            Some(
                unsafe { std::ffi::CStr::from_ptr(item.ifa_name) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }) else {
            current = item.ifa_next;
            continue;
        };
        if let Some(mac) = unix_link_address(item.ifa_addr) {
            macs.entry(name).or_insert(mac);
        }
        current = item.ifa_next;
    }

    let mut interfaces = BTreeMap::<String, Vec<Value>>::new();
    let mut current = head;
    while !current.is_null() {
        // SAFETY: current is a node in the getifaddrs list.
        let item = unsafe { &*current };
        let Some(name) = (if item.ifa_name.is_null() {
            None
        } else {
            // SAFETY: ifa_name is a NUL-terminated interface name.
            Some(
                unsafe { std::ffi::CStr::from_ptr(item.ifa_name) }
                    .to_string_lossy()
                    .into_owned(),
            )
        }) else {
            current = item.ifa_next;
            continue;
        };
        if let Some((address, scope)) = unix_ip_address(item.ifa_addr) {
            let internal = item.ifa_flags as i32 & libc::IFF_LOOPBACK != 0 || address.is_loopback();
            let mac = macs.get(&name).cloned().unwrap_or_else(zero_mac);
            match address {
                IpAddr::V4(address) => {
                    let mask = unix_ipv4_netmask(item.ifa_netmask).unwrap_or(Ipv4Addr::UNSPECIFIED);
                    let prefix = ipv4_prefix(mask);
                    interfaces.entry(name).or_default().push(json!({
                        "address": address.to_string(),
                        "netmask": mask.to_string(),
                        "family": "IPv4",
                        "mac": mac,
                        "internal": internal,
                        "cidr": prefix.map(|prefix| format!("{address}/{prefix}")),
                    }));
                }
                IpAddr::V6(address) => {
                    let prefix = ipv6_prefix(item.ifa_netmask);
                    let netmask = ipv6_mask(prefix.unwrap_or(0));
                    let address = if scope > 0 {
                        format!("{address}%{scope}")
                    } else {
                        address.to_string()
                    };
                    let cidr = prefix.map(|prefix| format!("{address}/{prefix}"));
                    interfaces.entry(name).or_default().push(json!({
                        "address": address,
                        "netmask": netmask.to_string(),
                        "family": "IPv6",
                        "mac": mac,
                        "internal": internal,
                        "cidr": cidr,
                        "scopeid": scope,
                    }));
                }
            }
        }
        current = item.ifa_next;
    }
    // SAFETY: head is the exact list returned by getifaddrs.
    unsafe { libc::freeifaddrs(head) };
    Ok(json!(interfaces))
}

#[cfg(unix)]
fn unix_ip_address(address: *const libc::sockaddr) -> Option<(IpAddr, u32)> {
    if address.is_null() {
        return None;
    }
    // SAFETY: getifaddrs sockaddr begins with the family field.
    match unsafe { (*address).sa_family as i32 } {
        libc::AF_INET => {
            // SAFETY: AF_INET addresses have a sockaddr_in layout.
            let address = unsafe { &*(address.cast::<libc::sockaddr_in>()) };
            Some((
                IpAddr::V4(Ipv4Addr::from(address.sin_addr.s_addr.to_ne_bytes())),
                0,
            ))
        }
        libc::AF_INET6 => {
            // SAFETY: AF_INET6 addresses have a sockaddr_in6 layout.
            let address = unsafe { &*(address.cast::<libc::sockaddr_in6>()) };
            Some((
                IpAddr::V6(Ipv6Addr::from(address.sin6_addr.s6_addr)),
                address.sin6_scope_id,
            ))
        }
        _ => None,
    }
}

#[cfg(unix)]
fn unix_ipv4_netmask(mask: *const libc::sockaddr) -> Option<Ipv4Addr> {
    if mask.is_null() {
        return None;
    }
    // SAFETY: an interface AF_INET netmask has a sockaddr_in layout.
    let mask = unsafe { &*(mask.cast::<libc::sockaddr_in>()) };
    Some(Ipv4Addr::from(mask.sin_addr.s_addr.to_ne_bytes()))
}

#[cfg(unix)]
fn ipv6_prefix(mask: *const libc::sockaddr) -> Option<u8> {
    if mask.is_null() {
        return None;
    }
    // SAFETY: an AF_INET6 netmask has a sockaddr_in6 layout.
    let bytes = unsafe { &*(mask.cast::<libc::sockaddr_in6>()) }
        .sin6_addr
        .s6_addr;
    mask_prefix(&bytes)
}

#[cfg(unix)]
fn unix_link_address(address: *const libc::sockaddr) -> Option<String> {
    if address.is_null() {
        return None;
    }
    // SAFETY: getifaddrs sockaddr begins with the family field.
    let family = unsafe { (*address).sa_family as i32 };
    #[cfg(target_os = "linux")]
    if family == libc::AF_PACKET {
        // SAFETY: AF_PACKET addresses have a sockaddr_ll layout.
        let link = unsafe { &*(address.cast::<libc::sockaddr_ll>()) };
        let bytes = &link.sll_addr[..(link.sll_halen as usize).min(link.sll_addr.len())];
        return (!bytes.is_empty()).then(|| format_mac(bytes));
    }
    #[cfg(target_os = "macos")]
    if family == libc::AF_LINK {
        // SAFETY: AF_LINK addresses have a sockaddr_dl layout.
        let link = unsafe { &*(address.cast::<libc::sockaddr_dl>()) };
        let start = link.sdl_nlen as usize;
        let end = start
            .saturating_add(link.sdl_alen as usize)
            .min(link.sdl_data.len());
        if start < end {
            let bytes: Vec<u8> = link.sdl_data[start..end]
                .iter()
                .map(|byte| *byte as u8)
                .collect();
            return Some(format_mac(&bytes));
        }
    }
    let _ = family;
    None
}

#[cfg(unix)]
fn ipv4_prefix(mask: Ipv4Addr) -> Option<u8> {
    mask_prefix(&mask.octets())
}

#[cfg(any(unix, test))]
fn mask_prefix(bytes: &[u8]) -> Option<u8> {
    let mut prefix = 0u8;
    let mut saw_zero = false;
    for byte in bytes {
        for bit in (0..8).rev() {
            let is_one = *byte & (1u8 << bit) != 0;
            if saw_zero && is_one {
                return None;
            }
            if is_one {
                prefix += 1;
            } else {
                saw_zero = true;
            }
        }
    }
    Some(prefix)
}

fn ipv6_mask(prefix: u8) -> Ipv6Addr {
    let mut bytes = [0u8; 16];
    for bit in 0..prefix.min(128) {
        bytes[(bit / 8) as usize] |= 1 << (7 - bit % 8);
    }
    Ipv6Addr::from(bytes)
}

#[cfg(unix)]
fn format_mac(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join(":")
}

fn zero_mac() -> String {
    "00:00:00:00:00:00".to_string()
}

#[cfg(target_os = "macos")]
fn macos_dns_servers() -> Result<Value> {
    use std::process::Command;
    let output = Command::new("/usr/sbin/scutil")
        .arg("--dns")
        .output()
        .context("read macOS system DNS configuration")?;
    if !output.status.success() {
        bail!("scutil --dns exited with {}", output.status);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    Ok(json!(parse_scutil_dns_output(&text)))
}

#[cfg(target_os = "linux")]
fn linux_dns_servers() -> Result<Value> {
    let contents = match std::fs::read_to_string("/etc/resolv.conf") {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(json!([])),
        Err(error) => return Err(error).context("read Linux resolver configuration"),
    };
    Ok(json!(parse_resolv_conf(&contents)))
}

#[cfg(any(target_os = "macos", test))]
fn parse_scutil_dns_output(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| line.trim().strip_prefix("nameserver["))
        .filter_map(|line| line.split_once(':').map(|(_, value)| value.trim()))
        .filter_map(|address| address.parse::<IpAddr>().ok())
        .map(|address| address.to_string())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect()
}

#[cfg(any(target_os = "linux", test))]
fn parse_resolv_conf(contents: &str) -> Vec<String> {
    contents
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == "nameserver")
                .then(|| fields.next())
                .flatten()
        })
        .filter_map(|address| address.parse::<IpAddr>().ok())
        .map(|address| address.to_string())
        .collect()
}

#[cfg(target_os = "windows")]
struct WindowsAddress {
    ip: IpAddr,
    prefix: u8,
    scope: u32,
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct WindowsAdapter {
    name: String,
    mac: String,
    internal: bool,
    addresses: Vec<WindowsAddress>,
    dns_servers: Vec<String>,
}

#[cfg(target_os = "windows")]
fn windows_adapters() -> Result<Vec<WindowsAdapter>> {
    use windows::Win32::Foundation::{ERROR_BUFFER_OVERFLOW, ERROR_NO_DATA};
    use windows::Win32::NetworkManagement::IpHelper::{
        GAA_FLAG_INCLUDE_PREFIX, GetAdaptersAddresses, IP_ADAPTER_ADDRESSES_LH,
    };
    use windows::Win32::Networking::WinSock::AF_UNSPEC;

    let mut size = 0u32;
    // SAFETY: null output requests the required size and does not dereference it.
    let status = unsafe {
        GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            None,
            &mut size,
        )
    };
    if status == ERROR_NO_DATA.0 {
        return Ok(Vec::new());
    }
    if status != ERROR_BUFFER_OVERFLOW.0 || size == 0 {
        bail!("GetAdaptersAddresses size query failed with Windows error {status}");
    }
    let word_count = (size as usize).div_ceil(std::mem::size_of::<usize>());
    let mut storage = vec![0usize; word_count];
    // SAFETY: usize storage has suitable alignment for the Windows adapter structs.
    let first = storage.as_mut_ptr().cast::<IP_ADAPTER_ADDRESSES_LH>();
    // SAFETY: storage is at least `size` bytes and is suitably aligned.
    let status = unsafe {
        GetAdaptersAddresses(
            AF_UNSPEC.0 as u32,
            GAA_FLAG_INCLUDE_PREFIX,
            None,
            Some(first),
            &mut size,
        )
    };
    if status != 0 {
        bail!("GetAdaptersAddresses failed with Windows error {status}");
    }

    let mut adapters = Vec::new();
    let mut current = first;
    while !current.is_null() {
        // SAFETY: current is a node in the OS-created GetAdaptersAddresses list.
        let adapter = unsafe { &*current };
        let name = if adapter.FriendlyName.0.is_null() {
            String::new()
        } else {
            let mut length = 0usize;
            // SAFETY: Windows provides a NUL-terminated UTF-16 FriendlyName.
            unsafe {
                while *adapter.FriendlyName.0.add(length) != 0 {
                    length += 1;
                }
                String::from_utf16_lossy(std::slice::from_raw_parts(adapter.FriendlyName.0, length))
            }
        };
        let mac_bytes = &adapter.PhysicalAddress
            [..(adapter.PhysicalAddressLength as usize).min(adapter.PhysicalAddress.len())];
        let mac = if mac_bytes.is_empty() {
            zero_mac()
        } else {
            mac_bytes
                .iter()
                .map(|byte| format!("{byte:02x}"))
                .collect::<Vec<_>>()
                .join(":")
        };
        let mut decoded = WindowsAdapter {
            name,
            mac,
            internal: adapter.IfType == 24,
            ..Default::default()
        };

        let mut unicast = adapter.FirstUnicastAddress;
        while !unicast.is_null() {
            // SAFETY: unicast is an OS-owned linked-list node.
            let entry = unsafe { &*unicast };
            if let Some(ip) = windows_socket_address(entry.Address) {
                decoded.internal |= ip.ip.is_loopback();
                decoded.addresses.push(WindowsAddress {
                    ip: ip.ip,
                    prefix: entry.OnLinkPrefixLength,
                    scope: ip.scope,
                });
            }
            unicast = entry.Next;
        }

        let mut dns = adapter.FirstDnsServerAddress;
        while !dns.is_null() {
            // SAFETY: dns is an OS-owned linked-list node.
            let entry = unsafe { &*dns };
            if let Some(address) = windows_socket_address(entry.Address) {
                decoded.dns_servers.push(address.format());
            }
            dns = entry.Next;
        }

        adapters.push(decoded);
        current = adapter.Next;
    }
    Ok(adapters)
}

#[cfg(target_os = "windows")]
struct WindowsIp {
    ip: IpAddr,
    scope: u32,
}

#[cfg(target_os = "windows")]
impl WindowsIp {
    fn format(&self) -> String {
        match self.ip {
            IpAddr::V6(ip) if self.scope > 0 => format!("{ip}%{}", self.scope),
            _ => self.ip.to_string(),
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_socket_address(
    address: windows::Win32::Networking::WinSock::SOCKET_ADDRESS,
) -> Option<WindowsIp> {
    use windows::Win32::Networking::WinSock::{AF_INET, AF_INET6, SOCKADDR_IN, SOCKADDR_IN6};
    if address.lpSockaddr.is_null() {
        return None;
    }
    if address.iSockaddrLength < 0 {
        return None;
    }
    // SAFETY: SOCKET_ADDRESS.lpSockaddr points to a sockaddr with family first.
    match unsafe { (*address.lpSockaddr).sa_family } {
        family
            if family == AF_INET
                && address.iSockaddrLength as usize >= std::mem::size_of::<SOCKADDR_IN>() =>
        {
            // SAFETY: AF_INET pointer and length match SOCKADDR_IN.
            let addr = unsafe { &*(address.lpSockaddr.cast::<SOCKADDR_IN>()) };
            // SAFETY: S_addr is the active IN_ADDR union member for IPv4.
            let octets = unsafe { addr.sin_addr.S_un.S_addr.to_ne_bytes() };
            Some(WindowsIp {
                ip: IpAddr::V4(Ipv4Addr::from(octets)),
                scope: 0,
            })
        }
        family
            if family == AF_INET6
                && address.iSockaddrLength as usize >= std::mem::size_of::<SOCKADDR_IN6>() =>
        {
            // SAFETY: AF_INET6 pointer and length match SOCKADDR_IN6.
            let addr = unsafe { &*(address.lpSockaddr.cast::<SOCKADDR_IN6>()) };
            // SAFETY: Byte is a valid view of the IN6_ADDR union.
            let octets = unsafe { addr.sin6_addr.u.Byte };
            // SAFETY: sin6_scope_id is the active scope representation.
            let scope = unsafe { addr.Anonymous.sin6_scope_id };
            Some(WindowsIp {
                ip: IpAddr::V6(Ipv6Addr::from(octets)),
                scope,
            })
        }
        _ => None,
    }
}

#[cfg(target_os = "windows")]
fn windows_network_interfaces() -> Result<Value> {
    let mut interfaces = std::collections::BTreeMap::<String, Vec<Value>>::new();
    for adapter in windows_adapters()? {
        let interface_name = adapter.name.clone();
        let mac = adapter.mac.clone();
        let internal = adapter.internal;
        for address in adapter.addresses {
            let (family, netmask, address_text, cidr) = match address.ip {
                IpAddr::V4(ip) => {
                    let prefix = address.prefix.min(32);
                    let mask = ipv4_mask(prefix);
                    (
                        "IPv4",
                        mask.to_string(),
                        ip.to_string(),
                        Some(format!("{ip}/{prefix}")),
                    )
                }
                IpAddr::V6(ip) => {
                    let prefix = address.prefix.min(128);
                    let mask = ipv6_mask(prefix);
                    let value = if address.scope > 0 {
                        format!("{ip}%{}", address.scope)
                    } else {
                        ip.to_string()
                    };
                    (
                        "IPv6",
                        mask.to_string(),
                        value.clone(),
                        Some(format!("{value}/{prefix}")),
                    )
                }
            };
            let mut value = json!({
                "address": address_text,
                "netmask": netmask,
                "family": family,
                "mac": mac.clone(),
                "internal": internal,
                "cidr": cidr,
            });
            if family == "IPv6" {
                value["scopeid"] = json!(address.scope);
            }
            interfaces
                .entry(interface_name.clone())
                .or_default()
                .push(value);
        }
    }
    Ok(json!(interfaces))
}

#[cfg(target_os = "windows")]
fn windows_dns_servers() -> Result<Value> {
    let servers = windows_adapters()?
        .into_iter()
        .flat_map(|adapter| adapter.dns_servers)
        .collect::<std::collections::BTreeSet<_>>();
    Ok(json!(servers))
}

#[cfg(target_os = "windows")]
fn ipv4_mask(prefix: u8) -> Ipv4Addr {
    let bits = if prefix == 0 {
        0
    } else {
        u32::MAX << (32 - prefix.min(32))
    };
    Ipv4Addr::from(bits.to_be_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linux_cpu_parser_skips_aggregate_and_maps_kernel_counters() {
        assert_eq!(parse_linux_cpu_line("cpu 10 20 30 40 50 60 70"), None);
        assert_eq!(
            parse_linux_cpu_line("cpu2 10 20 30 40 50 60 70"),
            Some([10, 20, 30, 40, 50, 60, 70])
        );
        assert_eq!(milliseconds_from_ticks(250, 100), 2500);
        assert_eq!(
            parse_linux_cpuinfo("model name : Example\ncpu MHz : 2400.5"),
            ("Example".into(), 2400)
        );
    }

    #[test]
    fn network_masks_require_contiguous_bits() {
        assert_eq!(mask_prefix(&[255, 255, 254, 0]), Some(23));
        assert_eq!(mask_prefix(&[255, 0, 255, 0]), None);
        assert_eq!(
            ipv6_mask(64),
            "ffff:ffff:ffff:ffff::".parse::<Ipv6Addr>().unwrap()
        );
    }

    #[test]
    fn resolver_config_parsers_accept_only_valid_system_addresses() {
        assert_eq!(
            parse_scutil_dns_output(
                "resolver #1\n nameserver[0] : 2001:db8::53\n nameserver[1] : 192.0.2.53\n nameserver[0] : 192.0.2.53"
            ),
            ["192.0.2.53", "2001:db8::53"]
        );
        assert_eq!(
            parse_resolv_conf(
                "search example.test\nnameserver 192.0.2.1\nnameserver bad\nnameserver fd00::1 # local"
            ),
            ["192.0.2.1", "fd00::1"]
        );
    }

    #[test]
    fn lookup_uses_system_host_resolution_and_honors_family_and_all() {
        let addresses = dns_lookup("127.0.0.1", 4, true).unwrap();
        assert_eq!(addresses, json!([{ "address": "127.0.0.1", "family": 4 }]));
        assert!(
            dns_lookup("127.0.0.1", 6, false)
                .unwrap_err()
                .to_string()
                .contains("ENOTFOUND")
        );
        assert!(
            dns_lookup("", 0, false)
                .unwrap_err()
                .to_string()
                .contains("EINVAL")
        );
    }

    #[test]
    fn live_queries_return_current_os_shapes() {
        let processors = cpus().unwrap();
        assert!(processors.is_array());
        assert!(!processors.as_array().unwrap().is_empty());
        let cpu = &processors[0];
        assert!(cpu["model"].is_string());
        assert!(cpu["speed"].is_number());
        for time in ["user", "nice", "sys", "idle", "irq"] {
            assert!(cpu["times"][time].is_number());
        }
        assert!(free_memory().unwrap() > 0);
        let first_uptime = uptime().unwrap();
        let second_uptime = uptime().unwrap();
        assert!(first_uptime > 0.0 && second_uptime >= first_uptime);
        assert!(network_interfaces().unwrap().is_object());
        assert!(dns_servers().unwrap().is_array());
        assert!(total_memory().unwrap_or_default() > 0);
        assert!(hostname().is_some());
    }
}
