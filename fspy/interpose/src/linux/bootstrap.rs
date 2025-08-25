use super::SYSCALL_MAGIC;
use seccompiler::{
    BpfProgram, SeccompAction, SeccompCondition, SeccompFilter, SeccompRule, apply_filter,
};

pub fn bootstrap() -> seccompiler::Result<()> {
    let syscalls_with_magic_indexes: &[(i64, u8)] = &[
        (libc::SYS_readlinkat, 4),
        (libc::SYS_openat, 4),
        (libc::SYS_execve, 3),
    ];
    let mut rules: std::collections::BTreeMap<i64, Vec<SeccompRule>> = syscalls_with_magic_indexes
        .iter()
        .cloned()
        .map(|(syscall, magic_index)| {
            Ok({
                (
                    syscall,
                    vec![SeccompRule::new(vec![SeccompCondition::new(
                        magic_index,
                        seccompiler::SeccompCmpArgLen::Qword,
                        seccompiler::SeccompCmpOp::Ne,
                        SYSCALL_MAGIC,
                    )?])?],
                )
            })
        })
        .collect::<seccompiler::Result<_>>()?;

    // trap sigaction registraion on SIGSYS
    rules.insert(
        libc::SYS_rt_sigaction,
        vec![SeccompRule::new(vec![
            // signum == SIGSYS
            SeccompCondition::new(
                0,
                seccompiler::SeccompCmpArgLen::Dword,
                seccompiler::SeccompCmpOp::Eq,
                libc::SIGSYS as _,
            )?,
            // sigaction != nullptr
            SeccompCondition::new(
                1,
                seccompiler::SeccompCmpArgLen::Qword,
                seccompiler::SeccompCmpOp::Ne,
                0,
            )?,
            SeccompCondition::new(
                4,
                seccompiler::SeccompCmpArgLen::Qword,
                seccompiler::SeccompCmpOp::Ne,
                SYSCALL_MAGIC,
            )?,
        ])?],
    );
    let filter = SeccompFilter::new(
        rules,
        SeccompAction::Allow,
        SeccompAction::Trap,
        std::env::consts::ARCH.try_into()?,
    )?;
    let filter = BpfProgram::try_from(filter)?;
    apply_filter(&filter)?;
    Ok(())
}
