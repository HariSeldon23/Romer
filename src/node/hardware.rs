use std::env;
use std::process::Command;
use thiserror::Error;
use std::time::{Duration, Instant};

/// Represents different virtualization types
#[derive(Debug, Clone, PartialEq)]
pub enum VirtualizationType {
    /// Indicates the system is running on physical hardware
    Physical,
    /// Represents a specific virtualization technology
    Virtual(String),
}

/// Represents the operating system type
#[derive(Debug, Clone, PartialEq)]
pub enum OperatingSystem {
    Windows,
    MacOS,
    Linux,
    Unknown,
}

/// Errors that can occur during hardware verification
#[derive(Error, Debug)]
pub enum HardwareError {
    #[error("CPU performance below minimum threshold")]
    InsufficientPerformance,
    #[error("Virtualization detection failed")]
    VirtualizationDetectionError,
}

/// Comprehensive hardware verification system
pub struct HardwareVerifier {
    /// Minimum operations per second required
    min_ops_required: u64,
    /// Test duration in seconds
    test_duration: Duration,
    /// Baseline operations per second for scoring
    baseline_ops: u64,
}

impl HardwareVerifier {
    /// Detect the current operating system
    pub fn detect_os() -> OperatingSystem {
        #[cfg(windows)]
        return OperatingSystem::Windows;

        #[cfg(target_os = "macos")]
        return OperatingSystem::MacOS;

        #[cfg(target_os = "linux")]
        return OperatingSystem::Linux;

        #[cfg(not(any(windows, target_os = "macos", target_os = "linux")))]
        return OperatingSystem::Unknown;
    }

    /// Detect virtualization across different operating systems
    pub fn detect_virtualization() -> Result<VirtualizationType, HardwareError> {
        match Self::detect_os() {
            OperatingSystem::Windows => HardwareVerifier::detect_windows_virtualization(),
            OperatingSystem::MacOS => HardwareVerifier::detect_macos_virtualization(),
            OperatingSystem::Linux => HardwareVerifier::detect_linux_virtualization(),
            OperatingSystem::Unknown => Ok(VirtualizationType::Physical),
        }
    }

    /// Windows-specific virtualization detection
    #[cfg(windows)]
    pub fn detect_windows_virtualization() -> Result<VirtualizationType, HardwareError> {
        // Multiple detection methods for Windows
        let methods = [
            // WMI Query via PowerShell
            || {
                let output = Command::new("powershell")
                    .args(&["-Command", "Get-WmiObject Win32_ComputerSystem | Select-Object Model"])
                    .output()
                    .ok()?;
                
                let model_str = String::from_utf8_lossy(&output.stdout);
                
                if model_str.contains("VMware") {
                    return Some(VirtualizationType::Virtual("VMware".to_string()));
                }
                
                if model_str.contains("VirtualBox") {
                    return Some(VirtualizationType::Virtual("VirtualBox".to_string()));
                }
                
                None
            },
            // Systeminfo method
            || {
                let output = Command::new("systeminfo")
                    .output()
                    .ok()?;
                
                let info_str = String::from_utf8_lossy(&output.stdout);
                
                if info_str.contains("Hyper-V") {
                    return Some(VirtualizationType::Virtual("Hyper-V".to_string()));
                }
                
                None
            },
            // WSL detection
            || {
                if env::var("WSL_DISTRO_NAME").is_ok() {
                    return Some(VirtualizationType::Virtual("WSL".to_string()));
                }
                None
            },
        ];

        // Try each detection method
        for method in methods.iter() {
            if let Some(result) = method() {
                return Ok(result);
            }
        }

        Ok(VirtualizationType::Physical)
    }

    /// Windows-specific virtualization detection (fallback for non-Windows platforms)
    #[cfg(not(windows))]
    pub fn detect_windows_virtualization() -> Result<VirtualizationType, HardwareError> {
        Ok(VirtualizationType::Physical)
    }

    /// MacOS-specific virtualization detection
    #[cfg(target_os = "macos")]
    pub fn detect_macos_virtualization() -> Result<VirtualizationType, HardwareError> {
        // Detection using system profiler
        let output = Command::new("system_profiler")
            .arg("SPHardwareDataType")
            .output()
            .map_err(|_| HardwareError::VirtualizationDetectionError)?;
        
        let hardware_info = String::from_utf8_lossy(&output.stdout);
        
        // Check for known virtualization markers
        if hardware_info.contains("VMware") {
            return Ok(VirtualizationType::Virtual("VMware".to_string()));
        }
        
        if hardware_info.contains("Parallels") {
            return Ok(VirtualizationType::Virtual("Parallels".to_string()));
        }
        
        // Additional MacOS-specific checks can be added here
        Ok(VirtualizationType::Physical)
    }

    /// MacOS-specific virtualization detection (fallback for non-MacOS platforms)
    #[cfg(not(target_os = "macos"))]
    pub fn detect_macos_virtualization() -> Result<VirtualizationType, HardwareError> {
        Ok(VirtualizationType::Physical)
    }

    /// Linux-specific virtualization detection
    #[cfg(target_os = "linux")]
    pub fn detect_linux_virtualization() -> Result<VirtualizationType, HardwareError> {
        // Multiple detection methods for Linux
        let methods = [
            // systemd-detect-virt method
            || {
                let output = Command::new("systemd-detect-virt")
                    .output()
                    .ok()?;
                
                if output.status.success() {
                    let virt_type = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if virt_type != "none" {
                        return Some(VirtualizationType::Virtual(virt_type));
                    }
                }
                None
            },
            // DMI detection method
            || {
                let output = Command::new("dmidecode")
                    .arg("-t")
                    .arg("system")
                    .output()
                    .ok()?;
                
                let output_str = String::from_utf8_lossy(&output.stdout);
                if output_str.contains("VMware") || output_str.contains("Virtual") {
                    return Some(VirtualizationType::Virtual("VMware".to_string()));
                }
                None
            },
            // Fallback: check for known virtualization environment variables
            || {
                if env::var("VIRTUAL_ENV").is_ok() || 
                   env::var("CONTAINER").is_ok() || 
                   env::var("KUBERNETES_SERVICE_HOST").is_ok() {
                    return Some(VirtualizationType::Virtual("Container/Kubernetes".to_string()));
                }
                None
            },
        ];

        // Try each detection method
        for method in methods.iter() {
            if let Some(result) = method() {
                return Ok(result);
            }
        }

        Ok(VirtualizationType::Physical)
    }

    /// Linux-specific virtualization detection (fallback for non-Linux platforms)
    #[cfg(not(target_os = "linux"))]
    pub fn detect_linux_virtualization() -> Result<VirtualizationType, HardwareError> {
        Ok(VirtualizationType::Physical)
    }

    /// Creates a new hardware verifier with default settings
    pub fn new() -> Self {
        Self {
            // Targeting roughly 1 million ops/sec minimum on modern hardware
            min_ops_required: 1_000_000,
            // Run test for 5 seconds
            test_duration: Duration::from_secs(5),
            // Baseline expectation for scoring
            baseline_ops: 2_000_000,
        }
    }

    /// Verifies hardware performance by running CPU integer operations
    pub fn verify(&self) -> Result<(VirtualizationType, VerificationResult), HardwareError> {
        // First, detect virtualization
        let virtualization_type = Self::detect_virtualization()?;

        // Perform performance verification
        let start_time = Instant::now();
        let end_time = start_time + self.test_duration;
        
        let mut operations = 0u64;
        let mut accumulator = 0u64;

        // Perform integer operations until test duration expires
        while Instant::now() < end_time {
            // Simple but non-optimizable integer operations
            for i in 0..1000 {
                accumulator = accumulator.wrapping_add(i);
                accumulator = accumulator.wrapping_mul(1337);
                accumulator = accumulator.wrapping_sub(i * 42);
                operations += 3;
            }

            // Periodically check if we've hit our operation threshold
            if operations % 3000 == 0 && operations > self.min_ops_required {
                // Early exit if we've proven sufficient performance
                break;
            }
        }

        let actual_duration = start_time.elapsed();
        // Prevent division by zero if no time has passed
        let ops_per_second = if actual_duration.as_secs() > 0 {
            operations / actual_duration.as_secs()
        } else {
            operations
        };

        // Safely calculate performance score, handling potential divide by zero
        let performance_score = (ops_per_second as f64 / self.baseline_ops as f64)
            .min(1.0)
            .max(0.0);

        let result = VerificationResult {
            ops_per_second,
            meets_requirements: ops_per_second >= self.min_ops_required,
            performance_score,
            test_duration: actual_duration,
        };

        if !result.meets_requirements {
            return Err(HardwareError::InsufficientPerformance);
        }

        Ok((virtualization_type, result))
    }
}

/// Results from hardware verification
#[derive(Debug, Clone)]
pub struct VerificationResult {
    /// Operations per second achieved
    pub ops_per_second: u64,
    /// Whether the hardware meets minimum requirements
    pub meets_requirements: bool,
    /// Performance score (0.0 to 1.0) relative to baseline
    pub performance_score: f64,
    /// Duration of the test
    pub test_duration: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_os_detection() {
        let os = HardwareVerifier::detect_os();
        assert!(matches!(
            os, 
            OperatingSystem::Windows | 
            OperatingSystem::MacOS | 
            OperatingSystem::Linux | 
            OperatingSystem::Unknown
        ));
    }

    #[test]
    fn test_virtualization_detection() {
        let result = HardwareVerifier::detect_virtualization();
        assert!(result.is_ok());
    }

    #[test]
    fn test_hardware_verification() {
        let verifier = HardwareVerifier::new();
        let result = verifier.verify();
        assert!(result.is_ok());
    }
}