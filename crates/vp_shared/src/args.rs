/// Restore the leading `--` that clap consumes before a trailing positional.
pub fn restore_leading_arg_separator(original: &[String], parsed: &mut Vec<String>) {
    // A trailing positional keeps every value in order. If the token just
    // before that suffix is a separator, it belongs to the forwarded args.
    let separator = original
        .strip_suffix(parsed.as_slice())
        .and_then(|prefix| prefix.last())
        .filter(|arg| arg.as_str() == "--");
    if let Some(separator) = separator {
        parsed.insert(0, separator.clone());
    }
}
