//! Start suspended, bind the worker to a kill-on-close job, then resume its thread.
use anyhow::{Result, ensure};
use std::{
    mem::{size_of, zeroed},
    os::windows::io::{AsRawHandle, FromRawHandle, OwnedHandle},
};
use tokio::process::Child;
use windows_sys::Win32::{
    Foundation::{CloseHandle, INVALID_HANDLE_VALUE},
    System::{Diagnostics::ToolHelp::*, JobObjects::*, Threading::*},
};
pub(super) struct Tree {
    job: Option<OwnedHandle>,
}
impl Tree {
    pub(super) fn new(child: &Child) -> Result<Self> {
        unsafe {
            let job = CreateJobObjectW(std::ptr::null(), std::ptr::null());
            ensure!(!job.is_null(), "Не удалось создать группу проверки Google");
            let job = OwnedHandle::from_raw_handle(job);
            let mut limits: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = zeroed();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            ensure!(
                SetInformationJobObject(
                    job.as_raw_handle(),
                    JobObjectExtendedLimitInformation,
                    (&limits as *const JOBOBJECT_EXTENDED_LIMIT_INFORMATION).cast(),
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32
                ) != 0,
                "Не удалось ограничить группу проверки Google"
            );
            let process = child
                .raw_handle()
                .ok_or_else(|| anyhow::anyhow!("Нет процесса проверки Google"))?;
            ensure!(
                AssignProcessToJobObject(job.as_raw_handle(), process) != 0,
                "Не удалось привязать процесс проверки Google"
            );
            let pid = child
                .id()
                .ok_or_else(|| anyhow::anyhow!("Нет идентификатора проверки Google"))?;
            let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0);
            ensure!(
                snapshot != INVALID_HANDLE_VALUE,
                "Не удалось найти поток проверки Google"
            );
            let mut entry: THREADENTRY32 = zeroed();
            entry.dwSize = size_of::<THREADENTRY32>() as u32;
            let mut found = false;
            let mut okay = Thread32First(snapshot, &mut entry) != 0;
            while okay {
                if entry.th32OwnerProcessID == pid {
                    let thread = OpenThread(THREAD_SUSPEND_RESUME, 0, entry.th32ThreadID);
                    if !thread.is_null() {
                        let prior = ResumeThread(thread);
                        found |= prior > 0 && prior != u32::MAX;
                        CloseHandle(thread);
                    }
                }
                okay = Thread32Next(snapshot, &mut entry) != 0;
            }
            CloseHandle(snapshot);
            ensure!(found, "Не удалось запустить поток проверки Google");
            Ok(Self { job: Some(job) })
        }
    }
    pub(super) fn kill(&mut self) {
        self.job.take();
    }
}
