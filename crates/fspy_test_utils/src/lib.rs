use std::{env::current_exe, process::Command};

use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use bincode::{Decode, Encode, config};

/// Creates a `std::process::Command` that only executes the provided function.
///
/// - $arg: The argument to pass to the function, must implement `Encode` and `Decode`.
/// - $f: The function to run in the separate process, takes one argument of the type of $arg.
#[macro_export]
macro_rules! command_executing {
    ($arg: expr, $f: expr) => {{
        // Generate a unique ID for every invocation of this macro.
        const ID: &str =
            ::core::concat!(::core::file!(), ":", ::core::line!(), ":", ::core::column!());

        fn assert_arg_type<A>(_arg: &A, _f: impl FnOnce(A)) {}
        assert_arg_type(&$arg, $f);

        // Register an initializer that runs the provided function when the process is started
        #[ctor::ctor]
        unsafe fn init() {
            $crate::init_impl(ID, $f);
        }
        // Create the command
        $crate::create_command(ID, $arg)
    }};
}

#[doc(hidden)]
pub fn init_impl<A: Decode<()>>(expected_id: &str, f: impl FnOnce(A)) {
    let mut args = ::std::env::args();
    // <test_binary> <expected_id> <arg_base64>
    let (Some(_program), Some(current_id), Some(arg_base64)) =
        (args.next(), args.next(), args.next())
    else {
        return;
    };
    if current_id != expected_id {
        return;
    }
    let arg_bytes = BASE64_STANDARD_NO_PAD.decode(arg_base64).expect("Failed to decode base64 arg");
    let arg: A = bincode::decode_from_slice(&arg_bytes, config::standard())
        .expect("Failed to decode bincode arg")
        .0;
    f(arg);
    std::process::exit(0);
}

#[doc(hidden)]
pub fn create_command(id: &str, arg: impl Encode) -> Command {
    let mut command = Command::new(current_exe().unwrap());
    let arg_bytes = bincode::encode_to_vec(&arg, config::standard()).expect("Failed to encode arg");
    let arg_base64 = BASE64_STANDARD_NO_PAD.encode(&arg_bytes);
    command.arg(id).arg(arg_base64);

    command
}

#[cfg(test)]
mod tests {
    use std::str::from_utf8;

    use super::*;

    #[test]
    fn test_command_executing() {
        let mut command = command_executing!(42u32, |arg: u32| {
            print!("{}", arg);
        });
        let output = command.output().unwrap();
        assert_eq!(from_utf8(&output.stdout), Ok("42"));
        assert!(output.status.success());
    }
}
