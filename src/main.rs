use std::{process::Command, os::unix::process::CommandExt};


fn main() {
    let mut cmd = Command::new("deno");
    cmd.arg("run").arg("-A").arg("main.ts");
    cmd.env("PORT", "3000");
    unsafe {
        cmd.pre_exec(|| {
            let ret = libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM);
            if ret != 0 {
                panic!("prctl failed");
            }
            Ok(())
        });
    }
    let mut child = cmd.spawn().unwrap();

    std::thread::spawn(move || {
        child.wait().unwrap();
    });
}
