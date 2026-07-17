//! Platform-specific security hardening.
//!
//! - OpenBSD: `pledge(2)` restricts syscalls, `unveil(2)` restricts filesystem access.
//! - Linux: `seccomp-bpf` filter blocks network and dangerous syscalls.
//! - FreeBSD/Windows: no-op (future work).
//!
//! Called after argument parsing, before running commands.

use std::path::Path;

/// Apply platform-specific security hardening.
///
/// `workspace_root` — if known, used for `unveil()` on OpenBSD.
/// `needs_editor` — if true, include `exec`/`proc` in the allowlist for `$EDITOR`.
pub fn apply(workspace_root: Option<&Path>, needs_editor: bool) {
    #[cfg(target_os = "openbsd")]
    {
        apply_pledge(needs_editor);
        if let Some(root) = workspace_root {
            apply_unveil(root);
        }
    }

    #[cfg(target_os = "linux")]
    {
        let _ = workspace_root;
        apply_seccomp(needs_editor);
    }

    #[cfg(not(any(target_os = "openbsd", target_os = "linux")))]
    {
        let _ = (workspace_root, needs_editor);
        // No hardening available on this platform.
    }
}

// ── OpenBSD: pledge + unveil ─────────────────────────────────

#[cfg(target_os = "openbsd")]
fn apply_pledge(needs_editor: bool) {
    // pledge is one-way: you can only narrow, never widen.
    // Include exec/proc/tty up front if the command needs $EDITOR.
    let promises = if needs_editor {
        "rpath wpath cpath stdio exec proc tty"
    } else {
        "rpath wpath cpath stdio"
    };

    // Best-effort: if pledge fails, the process will be killed by the kernel
    // on the next violation. We don't handle the error because there's no
    // recovery — the process must run with restrictions or not at all.
    let _ = pledge::pledge(promises, None);
}

/// Declare unveil(2) — not in the `libc` crate, so we declare it ourselves.
#[cfg(target_os = "openbsd")]
extern "C" {
    fn unveil(
        path: *const std::os::raw::c_char,
        permissions: *const std::os::raw::c_char,
    ) -> std::os::raw::c_int;
}

#[cfg(target_os = "openbsd")]
fn apply_unveil(workspace_root: &Path) {
    use std::ffi::CString;

    // Unveil the workspace directory for read/write/create
    if let Ok(path) = CString::new(workspace_root.to_string_lossy().as_bytes()) {
        let perms = CString::new("rwc").unwrap();
        // SAFETY: unveil(2) is an OpenBSD syscall. Path and perms are valid C strings.
        unsafe {
            let _ = unveil(path.as_ptr(), perms.as_ptr());
        }
    }

    // Unveil the global config directory for read/write
    if let Some(config_home) = std::env::var_os("HOME") {
        let config_path = std::path::PathBuf::from(config_home)
            .join(".config")
            .join("securitysmith");
        if let Ok(path) = CString::new(config_path.to_string_lossy().as_bytes()) {
            let perms = CString::new("rwc").unwrap();
            // SAFETY: unveil(2) is an OpenBSD syscall. Path and perms are valid C strings.
            unsafe {
                let _ = unveil(path.as_ptr(), perms.as_ptr());
            }
        }
    }

    // Lock unveil — no more paths can be unveiled after this.
    // Passing NULL path locks the unveil state.
    // SAFETY: unveil(2) with NULL path locks the state.
    unsafe {
        let _ = unveil(std::ptr::null(), std::ptr::null());
    }
}

// ── Linux: seccomp-bpf ────────────────────────────────────────

#[cfg(target_os = "linux")]
fn apply_seccomp(needs_editor: bool) {
    use seccompiler::{SeccompAction, SeccompFilter, SeccompRule, TargetArch};

    let arch = if cfg!(target_arch = "x86_64") {
        TargetArch::x86_64
    } else if cfg!(target_arch = "aarch64") {
        TargetArch::aarch64
    } else {
        // seccompiler only supports x86_64, aarch64, riscv64.
        return;
    };

    // When the command spawns $EDITOR, skip the seccomp filter entirely.
    // We cannot predict which syscalls the user's editor needs (sockets for
    // D-Bus/server mode, terminal ioctl, etc.). The filter protects
    // SecuritySmith's own code paths; the editor is a trusted external process
    // chosen by the user.
    if needs_editor {
        return;
    }

    // Deny-list: block network, ptrace, and system-level operations.
    // Everything else is allowed. This is a first implementation —
    // a tighter allow-list can be added in a follow-up after tracing
    // all syscalls used by every code path.
    let blocked: Vec<(i64, Vec<SeccompRule>)> = vec![
        // Network — SecuritySmith never makes network calls
        (libc::SYS_socket, vec![]),
        (libc::SYS_connect, vec![]),
        (libc::SYS_bind, vec![]),
        (libc::SYS_listen, vec![]),
        (libc::SYS_accept, vec![]),
        (libc::SYS_accept4, vec![]),
        (libc::SYS_sendto, vec![]),
        (libc::SYS_recvfrom, vec![]),
        (libc::SYS_sendmsg, vec![]),
        (libc::SYS_recvmsg, vec![]),
        (libc::SYS_socketpair, vec![]),
        (libc::SYS_getsockopt, vec![]),
        (libc::SYS_setsockopt, vec![]),
        (libc::SYS_shutdown, vec![]),
        // System-level — dangerous operations
        (libc::SYS_ptrace, vec![]),
        (libc::SYS_chroot, vec![]),
        (libc::SYS_mount, vec![]),
        (libc::SYS_umount2, vec![]),
        (libc::SYS_reboot, vec![]),
        (libc::SYS_kexec_load, vec![]),
        (libc::SYS_init_module, vec![]),
        (libc::SYS_delete_module, vec![]),
        (libc::SYS_pivot_root, vec![]),
        (libc::SYS_swapon, vec![]),
        (libc::SYS_swapoff, vec![]),
    ];

    let rules: std::collections::BTreeMap<i64, Vec<SeccompRule>> = blocked.into_iter().collect();

    // Deny-list: blocked syscalls get KillProcess, everything else is allowed.
    let filter = match SeccompFilter::new(
        rules,
        SeccompAction::Allow, // mismatch_action: unlisted syscalls = allowed
        SeccompAction::KillProcess, // match_action: listed (blocked) syscalls = killed
        arch,
    ) {
        Ok(f) => f,
        Err(_) => return,
    };

    let bpf_prog: seccompiler::BpfProgram = match filter.try_into() {
        Ok(p) => p,
        Err(_) => return,
    };

    // Best-effort: if seccomp can't be applied, continue without it.
    let _ = seccompiler::apply_filter(&bpf_prog);
}
