//! \u5185\u5b58\u4f18\u5316\uff08\u4e0d\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650\uff09
//!
//! \u505a\u4e00\u4ef6\u4e8b\uff1a\u679a\u4e3e\u7cfb\u7edf\u4e0a\u53ef\u8bbf\u95ee\u7684\u8fdb\u7a0b\uff0c\u5bf9\u5b83\u4eec\u8c03\u7528 EmptyWorkingSet\uff0c
//! \u628a\u5404\u8fdb\u7a0b\u7684\u5de5\u4f5c\u96c6\uff08\u7269\u7406\u5185\u5b58\u90e8\u5206\uff09\u4ea4\u8fd8\u7ed9\u7cfb\u7edf\u3002
//!
//! \u53c2\u8003\uff1amemreduct \u7684\u8fdb\u7a0b\u5de5\u4f5c\u96c6\u6e05\u7406\u3002
//! \u4e0d\u505a\u9700\u8981\u7ba1\u7406\u5458\u6743\u9650\u7684\u64cd\u4f5c\uff08\u5f85\u547d\u5217\u8868\u6e05\u7406\u3001\u7cfb\u7edf\u5de5\u4f5c\u96c6\u6e05\u7406\u7b49\uff09\u3002

use serde::Serialize;

#[derive(Debug, Clone, Serialize, Default)]
pub struct MemoryOptimizeResult {
	pub available_before_mb: u64,
	pub available_after_mb: u64,
	pub freed_mb: i64,
	pub trimmed_processes: u32,
}

#[cfg(windows)]
mod win {
	use std::ffi::c_void;

	#[repr(C)]
	pub struct MemoryStatusEx {
		pub length: u32,
		pub memory_load: u32,
		pub total_phys: u64,
		pub avail_phys: u64,
		pub total_page_file: u64,
		pub avail_page_file: u64,
		pub total_virtual: u64,
		pub avail_virtual: u64,
		pub avail_extended_virtual: u64,
	}

	#[repr(C)]
	pub struct ProcessEntry32W {
		pub dw_size: u32,
		pub cnt_usage: u32,
		pub th32_process_id: u32,
		pub th32_default_heap_id: usize,
		pub th32_module_id: u32,
		pub cnt_threads: u32,
		pub th32_parent_process_id: u32,
		pub pc_pri_class_base: i32,
		pub dw_flags: u32,
		pub sz_exe_file: [u16; 260],
	}

	pub const PROCESS_QUERY_INFORMATION: u32 = 0x0400;
	pub const PROCESS_SET_QUOTA: u32 = 0x0100;
	pub const TH32CS_SNAPPROCESS: u32 = 0x00000002;
	pub const INVALID_HANDLE_VALUE: isize = -1;

	unsafe extern "system" {
		pub fn GetCurrentProcessId() -> u32;
		pub fn OpenProcess(access: u32, inherit: i32, pid: u32) -> *mut c_void;
		pub fn CloseHandle(handle: *mut c_void) -> i32;
		pub fn GlobalMemoryStatusEx(status: *mut MemoryStatusEx) -> i32;
		pub fn CreateToolhelp32Snapshot(flags: u32, pid: u32) -> *mut c_void;
		pub fn Process32FirstW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
		pub fn Process32NextW(snapshot: *mut c_void, entry: *mut ProcessEntry32W) -> i32;
	}

	#[link(name = "psapi")]
	unsafe extern "system" {
		pub fn EmptyWorkingSet(process: *mut c_void) -> i32;
	}

	pub fn available_physical_mb() -> u64 {
		let mut status = MemoryStatusEx {
			length: std::mem::size_of::<MemoryStatusEx>() as u32,
			memory_load: 0,
			total_phys: 0,
			avail_phys: 0,
			total_page_file: 0,
			avail_page_file: 0,
			total_virtual: 0,
			avail_virtual: 0,
			avail_extended_virtual: 0,
		};
		let ok = unsafe { GlobalMemoryStatusEx(&mut status) };
		if ok == 0 {
			0
		} else {
			status.avail_phys / (1024 * 1024)
		}
	}

	/// \u5bf9\u6240\u6709\u53ef\u6253\u5f00\u7684\u8fdb\u7a0b\u8c03\u7528 EmptyWorkingSet\uff0c\u8fd4\u56de\u6210\u529f\u6570\u91cf
	pub fn trim_process_working_sets(own_pid: u32) -> u32 {
		let mut trimmed = 0u32;
		let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) };
		if snapshot.is_null() || snapshot as isize == INVALID_HANDLE_VALUE {
			return 0;
		}
		let mut entry = ProcessEntry32W {
			dw_size: std::mem::size_of::<ProcessEntry32W>() as u32,
			cnt_usage: 0,
			th32_process_id: 0,
			th32_default_heap_id: 0,
			th32_module_id: 0,
			cnt_threads: 0,
			th32_parent_process_id: 0,
			pc_pri_class_base: 0,
			dw_flags: 0,
			sz_exe_file: [0; 260],
		};
		let mut ok = unsafe { Process32FirstW(snapshot, &mut entry) };
		while ok != 0 {
			let pid = entry.th32_process_id;
			// \u8df3\u8fc7 Idle / System\uff0c\u4ee5\u53ca\u81ea\u5df1\uff08\u81ea\u5df1\u7684\u5de5\u4f5c\u96c6\u7559\u7ed9\u7a97\u53e3\u6e32\u67d3\uff09
			if pid > 4 && pid != own_pid {
				let handle = unsafe {
					OpenProcess(PROCESS_QUERY_INFORMATION | PROCESS_SET_QUOTA, 0, pid)
				};
				if !handle.is_null() {
					if unsafe { EmptyWorkingSet(handle) } != 0 {
						trimmed += 1;
					}
					unsafe { CloseHandle(handle) };
				}
			}
			ok = unsafe { Process32NextW(snapshot, &mut entry) };
		}
		unsafe { CloseHandle(snapshot) };
		trimmed
	}
}

#[cfg(windows)]
pub fn optimize_system_memory() -> MemoryOptimizeResult {
	use win::*;

	let before = available_physical_mb();
	let own_pid = unsafe { GetCurrentProcessId() };
	let trimmed_processes = trim_process_working_sets(own_pid);
	let after = available_physical_mb();

	MemoryOptimizeResult {
		available_before_mb: before,
		available_after_mb: after,
		freed_mb: after as i64 - before as i64,
		trimmed_processes,
	}
}

#[cfg(not(windows))]
pub fn optimize_system_memory() -> MemoryOptimizeResult {
	MemoryOptimizeResult::default()
}
