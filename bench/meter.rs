// meter <cmd> [args...]: runs cmd inside a Windows job object and prints its output followed by
// `@@METER {"wall":ms,"cpu":ms,"peak":bytes,"code":n}`, where cpu and peak cover the whole process tree.
use std::ffi::c_void;
use std::os::windows::io::AsRawHandle;
use std::process::{Command, Stdio};
use std::time::Instant;

#[repr(C)]
#[derive(Default)]
struct Basic { per_process_user: i64, per_job_user: i64, flags: u32, min_ws: usize, max_ws: usize, active: u32, affinity: usize, priority: u32, sched: u32 }
#[repr(C)]
#[derive(Default)]
struct Extended { basic: Basic, io: [u64; 6], process_limit: usize, job_limit: usize, peak_process: usize, peak_job: usize }
#[repr(C)]
#[derive(Default)]
struct Accounting { user: i64, kernel: i64, period_user: i64, period_kernel: i64, faults: u32, total: u32, active: u32, terminated: u32 }

#[link(name = "kernel32")]
unsafe extern "system" {
    fn CreateJobObjectW(attrs: *mut c_void, name: *const u16) -> *mut c_void;
    fn AssignProcessToJobObject(job: *mut c_void, process: *mut c_void) -> i32;
    fn QueryInformationJobObject(job: *mut c_void, class: i32, info: *mut c_void, len: u32, ret: *mut u32) -> i32;
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let job = unsafe { CreateJobObjectW(std::ptr::null_mut(), std::ptr::null()) };
    let t = Instant::now();
    let child = Command::new(&args[0]).args(&args[1..]).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn().expect("spawn");
    unsafe { AssignProcessToJobObject(job, child.as_raw_handle() as *mut c_void) };
    let out = child.wait_with_output().expect("wait");
    let wall = t.elapsed().as_secs_f64() * 1000.0;
    let mut ext = Extended::default();
    let mut acc = Accounting::default();
    unsafe {
        QueryInformationJobObject(job, 9, &mut ext as *mut _ as *mut c_void, std::mem::size_of::<Extended>() as u32, std::ptr::null_mut());
        QueryInformationJobObject(job, 1, &mut acc as *mut _ as *mut c_void, std::mem::size_of::<Accounting>() as u32, std::ptr::null_mut());
    }
    let cpu = (acc.user + acc.kernel) as f64 / 10_000.0;
    print!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    println!("@@METER {{\"wall\":{wall:.2},\"cpu\":{cpu:.2},\"peak\":{},\"code\":{}}}", ext.peak_job, out.status.code().unwrap_or(-1));
}
