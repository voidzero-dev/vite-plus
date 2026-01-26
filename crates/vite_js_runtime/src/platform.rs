use std::fmt;

use vite_str::Str;

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

    /// Get the platform string for Node.js distribution naming
    /// e.g., "linux-x64", "darwin-arm64", "win-x64"
    #[must_use]
    pub fn node_platform_string(self) -> Str {
        let os = match self.os {
            Os::Linux => "linux",
            Os::Darwin => "darwin",
            Os::Windows => "win",
        };
        let arch = match self.arch {
            Arch::X64 => "x64",
            Arch::Arm64 => "arm64",
        };
        vite_str::format!("{os}-{arch}")
    }

    /// Get the archive extension for this platform
    #[must_use]
    pub const fn archive_extension(self) -> &'static str {
        match self.os {
            Os::Windows => "zip",
            Os::Linux | Os::Darwin => "tar.gz",
        }
    }
}

impl fmt::Display for Platform {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.node_platform_string())
    }
}

impl Os {
    /// Detect the current operating system
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
    }
}

impl Arch {
    /// Detect the current CPU architecture
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_platform_detection() {
        let platform = Platform::current();

        // Just verify it doesn't panic and returns a valid platform
        let platform_str = platform.node_platform_string();
        assert!(!platform_str.is_empty());

        // Verify format is "os-arch"
        let parts: Vec<&str> = platform_str.split('-').collect();
        assert_eq!(parts.len(), 2);
    }

    #[test]
    fn test_node_platform_strings() {
        let cases = [
            (Platform { os: Os::Linux, arch: Arch::X64 }, "linux-x64"),
            (Platform { os: Os::Linux, arch: Arch::Arm64 }, "linux-arm64"),
            (Platform { os: Os::Darwin, arch: Arch::X64 }, "darwin-x64"),
            (Platform { os: Os::Darwin, arch: Arch::Arm64 }, "darwin-arm64"),
            (Platform { os: Os::Windows, arch: Arch::X64 }, "win-x64"),
            (Platform { os: Os::Windows, arch: Arch::Arm64 }, "win-arm64"),
        ];

        for (platform, expected) in cases {
            assert_eq!(platform.node_platform_string(), expected);
        }
    }

    #[test]
    fn test_archive_extension() {
        assert_eq!(Platform { os: Os::Linux, arch: Arch::X64 }.archive_extension(), "tar.gz");
        assert_eq!(Platform { os: Os::Darwin, arch: Arch::Arm64 }.archive_extension(), "tar.gz");
        assert_eq!(Platform { os: Os::Windows, arch: Arch::X64 }.archive_extension(), "zip");
    }
}
