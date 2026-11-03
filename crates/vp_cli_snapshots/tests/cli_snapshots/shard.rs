/// Split the deterministically ordered trials round-robin across CI jobs.
/// Apply this before libtest filters so filtered runs keep the same assignment.
pub fn select<T>(tests: Vec<T>, shard: &str) -> Result<Vec<T>, &'static str> {
    let (index, total) = shard
        .split_once('/')
        .and_then(|(index, total)| {
            Some((index.parse::<usize>().ok()?, total.parse::<usize>().ok()?))
        })
        .filter(|&(index, total)| index > 0 && index <= total)
        .ok_or("VP_SNAP_SHARD must be INDEX/TOTAL with 1 <= INDEX <= TOTAL")?;

    Ok(tests.into_iter().skip(index - 1).step_by(total).collect())
}
