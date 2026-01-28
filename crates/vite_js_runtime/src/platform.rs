use std::fmt;

/// Represents a platform (OS + architecture) combination
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

/// Operating system
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Darwin,
    Windows,
}

/// CPU architecture
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X64,
    Arm64,
}

impl Platform {
    /// Detect the current platform
    #[must_use]
    pub const fn current() -> Self {
        Self { os: Os::current(), arch: Arch::current() }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}-{}", self.os, self.arch)
    }
}

impl Os {
    /// Detect the current operating system.
    ///
    /// # Supported platforms
    /// - Linux (`target_os = "linux"`)
    /// - macOS (`target_os = "macos"`)
    /// - Windows (`target_os = "windows"`)
    ///
    /// Compilation will fail on unsupported operating systems.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_os = "linux")]
        {
            Self::Linux
        }
        #[cfg(target_os = "macos")]
        {
            Self::Darwin
        }
        #[cfg(target_os = "windows")]
        {
            Self::Windows
        }
        #[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
        {
            compile_error!(
                "Unsupported operating system. vite_js_runtime only supports Linux, macOS, and Windows."
            )
        }
    }
}

impl fmt::Display for Os {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Linux => write!(f, "linux"),
            Self::Darwin => write!(f, "darwin"),
            Self::Windows => write!(f, "windows"),
        }
    }
}

impl Arch {
    /// Detect the current CPU architecture.
    ///
    /// # Supported architectures
    /// - x86_64 (`target_arch = "x86_64"`)
    /// - ARM64/AArch64 (`target_arch = "aarch64"`)
    ///
    /// Compilation will fail on unsupported architectures.
    #[must_use]
    pub const fn current() -> Self {
        #[cfg(target_arch = "x86_64")]
        {
            Self::X64
        }
        #[cfg(target_arch = "aarch64")]
        {
            Self::Arm64
        }
        #[cfg(not(any(target_arch = "x86_64", target_arch = "aarch64")))]
        {
            compile_error!(
                "Unsupported CPU architecture. vite_js_runtime only supports x86_64 and aarch64."
            )
        }
    }
}

impl fmt::Display for Arch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::X64 => write!(f, "x64"),
            Self::Arm64 => write!(f, "arm64"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        // Just verify it doesn't panic and returns a valid platform
        let platform_str = platform.to_string();
        assert!(!platform_str.is_empty());

        // Verify format is "os-arch"
        let parts: Vec<&str> = platform_str.split('-').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_platform_display() {
        let cases = [
            (Platform { os: Os::Linux, arch: Arch::X64 }, "linux-x64"),
            (Platform { os: Os::Linux, arch: Arch::Arm64 }, "linux-arm64"),
            (Platform { os: Os::Darwin, arch: Arch::X64 }, "darwin-x64"),
            (Platform { os: Os::Darwin, arch: Arch::Arm64 }, "darwin-arm64"),
            (Platform { os: Os::Windows, arch: Arch::X64 }, "windows-x64"),
            (Platform { os: Os::Windows, arch: Arch::Arm64 }, "windows-arm64"),
        ];

        for (platform, expected) in cases {
            assert_eq!(platform.to_string(), expected);
        }
    }

    #[test]
    fn test_os_display() {
        assert_eq!(Os::Linux.to_string(), "linux");
        assert_eq!(Os::Darwin.to_string(), "darwin");
        assert_eq!(Os::Windows.to_string(), "windows");
    }

    #[test]
    fn test_arch_display() {
        assert_eq!(Arch::X64.to_string(), "x64");
        assert_eq!(Arch::Arm64.to_string(), "arm64");
    }
}
