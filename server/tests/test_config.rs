/// Test configuration system for optimizing integration test performance
/// across different environments (local, CI, production-like)
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct TestConfig {
    pub environment: TestEnvironment,
    pub scale: TestScale,
    pub timeouts: TestTimeouts,
    pub concurrency: ConcurrencyLimits,
    pub memory: MemoryLimits,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestEnvironment {
    Local,
    GitHubActions,
    AzureDevOps,
    Docker,
    Production,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestScale {
    Minimal, // Fastest tests, minimal coverage
    Small,   // Quick tests, good coverage
    Medium,  // Balanced tests
    Large,   // Comprehensive tests
    Stress,  // Performance stress tests
}

#[derive(Debug, Clone)]
pub struct TestTimeouts {
    pub short_operation: Duration,
    pub medium_operation: Duration,
    pub long_operation: Duration,
    pub total_test_timeout: Duration,
}

#[derive(Debug, Clone)]
pub struct ConcurrencyLimits {
    pub max_parallel_tasks: usize,
    pub max_concurrent_requests: usize,
    pub rate_limit_rps: u32,
}

#[derive(Debug, Clone)]
pub struct MemoryLimits {
    pub max_cache_entries: usize,
    pub max_test_data_size: usize,
    pub cache_ttl: Duration,
}

impl TestConfig {
    /// Detect environment and create appropriate configuration
    pub fn from_environment() -> Self {
        let environment = Self::detect_environment();
        Self::for_environment(environment)
    }

    /// Create configuration for specific environment
    pub fn for_environment(env: TestEnvironment) -> Self {
        match env {
            TestEnvironment::Local => Self::local_config(),
            TestEnvironment::GitHubActions => Self::github_actions_config(),
            TestEnvironment::AzureDevOps => Self::azure_devops_config(),
            TestEnvironment::Docker => Self::docker_config(),
            TestEnvironment::Production => Self::production_config(),
        }
    }

    /// Create configuration for specific scale
    pub fn for_scale(scale: TestScale) -> Self {
        let mut config = Self::local_config();
        config.scale = scale.clone();

        match scale {
            TestScale::Minimal => {
                config.timeouts.short_operation = Duration::from_millis(50);
                config.timeouts.medium_operation = Duration::from_millis(200);
                config.timeouts.long_operation = Duration::from_millis(500);
                config.timeouts.total_test_timeout = Duration::from_secs(5);
                config.concurrency.max_parallel_tasks = 2;
                config.memory.max_cache_entries = 10;
                config.memory.max_test_data_size = 100;
            }
            TestScale::Small => {
                config.timeouts.short_operation = Duration::from_millis(100);
                config.timeouts.medium_operation = Duration::from_millis(500);
                config.timeouts.long_operation = Duration::from_secs(2);
                config.timeouts.total_test_timeout = Duration::from_secs(30);
                config.concurrency.max_parallel_tasks = 5;
                config.memory.max_cache_entries = 50;
                config.memory.max_test_data_size = 500;
            }
            TestScale::Medium => {
                config.timeouts.short_operation = Duration::from_millis(200);
                config.timeouts.medium_operation = Duration::from_secs(2);
                config.timeouts.long_operation = Duration::from_secs(10);
                config.timeouts.total_test_timeout = Duration::from_secs(120);
                config.concurrency.max_parallel_tasks = 10;
                config.memory.max_cache_entries = 200;
                config.memory.max_test_data_size = 2000;
            }
            TestScale::Large => {
                config.timeouts.short_operation = Duration::from_millis(500);
                config.timeouts.medium_operation = Duration::from_secs(5);
                config.timeouts.long_operation = Duration::from_secs(30);
                config.timeouts.total_test_timeout = Duration::from_secs(600);
                config.concurrency.max_parallel_tasks = 20;
                config.memory.max_cache_entries = 1000;
                config.memory.max_test_data_size = 10000;
            }
            TestScale::Stress => {
                config.timeouts.short_operation = Duration::from_secs(1);
                config.timeouts.medium_operation = Duration::from_secs(10);
                config.timeouts.long_operation = Duration::from_secs(60);
                config.timeouts.total_test_timeout = Duration::from_secs(1800);
                config.concurrency.max_parallel_tasks = 50;
                config.memory.max_cache_entries = 5000;
                config.memory.max_test_data_size = 50000;
            }
        }

        config
    }

    /// Detect current environment from environment variables
    fn detect_environment() -> TestEnvironment {
        if std::env::var("GITHUB_ACTIONS").is_ok() {
            TestEnvironment::GitHubActions
        } else if std::env::var("AZURE_HTTP_USER_AGENT").is_ok() {
            TestEnvironment::AzureDevOps
        } else if std::env::var("DOCKER_ENV").is_ok()
            || std::path::Path::new("/.dockerenv").exists()
        {
            TestEnvironment::Docker
        } else if std::env::var("PRODUCTION_ENV").is_ok() {
            TestEnvironment::Production
        } else {
            TestEnvironment::Local
        }
    }

    fn local_config() -> Self {
        Self {
            environment: TestEnvironment::Local,
            scale: TestScale::Large,
            timeouts: TestTimeouts {
                short_operation: Duration::from_millis(100),
                medium_operation: Duration::from_secs(1),
                long_operation: Duration::from_secs(10),
                total_test_timeout: Duration::from_secs(300),
            },
            concurrency: ConcurrencyLimits {
                max_parallel_tasks: 10,
                max_concurrent_requests: 20,
                rate_limit_rps: 100,
            },
            memory: MemoryLimits {
                max_cache_entries: 1000,
                max_test_data_size: 10000,
                cache_ttl: Duration::from_secs(300),
            },
        }
    }

    fn github_actions_config() -> Self {
        Self {
            environment: TestEnvironment::GitHubActions,
            scale: TestScale::Small,
            timeouts: TestTimeouts {
                short_operation: Duration::from_millis(200),
                medium_operation: Duration::from_secs(2),
                long_operation: Duration::from_secs(15),
                total_test_timeout: Duration::from_secs(120),
            },
            concurrency: ConcurrencyLimits {
                max_parallel_tasks: 4, // Limited CPU cores
                max_concurrent_requests: 10,
                rate_limit_rps: 50,
            },
            memory: MemoryLimits {
                max_cache_entries: 100,
                max_test_data_size: 1000,
                cache_ttl: Duration::from_secs(60),
            },
        }
    }

    fn azure_devops_config() -> Self {
        Self {
            environment: TestEnvironment::AzureDevOps,
            scale: TestScale::Medium,
            timeouts: TestTimeouts {
                short_operation: Duration::from_millis(150),
                medium_operation: Duration::from_secs(3),
                long_operation: Duration::from_secs(20),
                total_test_timeout: Duration::from_secs(300),
            },
            concurrency: ConcurrencyLimits {
                max_parallel_tasks: 8, // Better resources than GitHub Actions
                max_concurrent_requests: 15,
                rate_limit_rps: 75,
            },
            memory: MemoryLimits {
                max_cache_entries: 500,
                max_test_data_size: 5000,
                cache_ttl: Duration::from_secs(180),
            },
        }
    }

    fn docker_config() -> Self {
        Self {
            environment: TestEnvironment::Docker,
            scale: TestScale::Small,
            timeouts: TestTimeouts {
                short_operation: Duration::from_millis(300),
                medium_operation: Duration::from_secs(5),
                long_operation: Duration::from_secs(30),
                total_test_timeout: Duration::from_secs(180),
            },
            concurrency: ConcurrencyLimits {
                max_parallel_tasks: 4, // Conservative for containers
                max_concurrent_requests: 8,
                rate_limit_rps: 30,
            },
            memory: MemoryLimits {
                max_cache_entries: 200,
                max_test_data_size: 2000,
                cache_ttl: Duration::from_secs(120),
            },
        }
    }

    fn production_config() -> Self {
        Self {
            environment: TestEnvironment::Production,
            scale: TestScale::Stress,
            timeouts: TestTimeouts {
                short_operation: Duration::from_millis(50),
                medium_operation: Duration::from_millis(500),
                long_operation: Duration::from_secs(5),
                total_test_timeout: Duration::from_secs(1800),
            },
            concurrency: ConcurrencyLimits {
                max_parallel_tasks: 50, // High performance environment
                max_concurrent_requests: 100,
                rate_limit_rps: 500,
            },
            memory: MemoryLimits {
                max_cache_entries: 10000,
                max_test_data_size: 100000,
                cache_ttl: Duration::from_secs(600),
            },
        }
    }

    /// Get test data generation parameters
    pub fn test_data_params(&self) -> TestDataParams {
        match self.scale {
            TestScale::Minimal => TestDataParams {
                subscription_count: 2,
                resource_groups_per_sub: 1,
                namespaces_per_sub: 1,
                max_iterations: 10,
            },
            TestScale::Small => TestDataParams {
                subscription_count: 5,
                resource_groups_per_sub: 3,
                namespaces_per_sub: 2,
                max_iterations: 50,
            },
            TestScale::Medium => TestDataParams {
                subscription_count: 20,
                resource_groups_per_sub: 5,
                namespaces_per_sub: 3,
                max_iterations: 200,
            },
            TestScale::Large => TestDataParams {
                subscription_count: 50,
                resource_groups_per_sub: 10,
                namespaces_per_sub: 5,
                max_iterations: 1000,
            },
            TestScale::Stress => TestDataParams {
                subscription_count: 200,
                resource_groups_per_sub: 20,
                namespaces_per_sub: 10,
                max_iterations: 10000,
            },
        }
    }

    /// Check if a specific test should run in current environment
    pub fn should_run_test(&self, test_category: TestCategory) -> bool {
        match (test_category, &self.scale) {
            (TestCategory::Unit, _) => true, // Always run unit tests
            (TestCategory::Integration, TestScale::Minimal) => false, // Skip in minimal mode
            (TestCategory::Integration, _) => true,
            (TestCategory::Performance, TestScale::Minimal | TestScale::Small) => false,
            (TestCategory::Performance, _) => true,
            (TestCategory::Stress, TestScale::Stress) => true,
            (TestCategory::Stress, _) => false, // Only run stress tests in stress mode
        }
    }

    /// Get environment-specific assertion parameters
    pub fn assertion_params(&self) -> AssertionParams {
        match self.environment {
            TestEnvironment::Local => AssertionParams {
                performance_tolerance: 1.0, // Strict performance requirements
                memory_tolerance: 1.0,
                timeout_multiplier: 1.0,
            },
            TestEnvironment::GitHubActions => AssertionParams {
                performance_tolerance: 2.0, // More lenient due to shared resources
                memory_tolerance: 1.5,
                timeout_multiplier: 2.0,
            },
            TestEnvironment::AzureDevOps => AssertionParams {
                performance_tolerance: 1.5,
                memory_tolerance: 1.2,
                timeout_multiplier: 1.5,
            },
            TestEnvironment::Docker => AssertionParams {
                performance_tolerance: 2.5, // Most lenient due to container overhead
                memory_tolerance: 2.0,
                timeout_multiplier: 3.0,
            },
            TestEnvironment::Production => AssertionParams {
                performance_tolerance: 0.8, // Strictest requirements
                memory_tolerance: 0.9,
                timeout_multiplier: 0.8,
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct TestDataParams {
    pub subscription_count: usize,
    pub resource_groups_per_sub: usize,
    pub namespaces_per_sub: usize,
    pub max_iterations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TestCategory {
    Unit,
    Integration,
    Performance,
    Stress,
}

#[derive(Debug, Clone)]
pub struct AssertionParams {
    pub performance_tolerance: f64, // Multiplier for performance expectations
    pub memory_tolerance: f64,      // Multiplier for memory expectations
    pub timeout_multiplier: f64,    // Multiplier for timeout values
}

/// Utility macros for environment-aware testing
#[macro_export]
macro_rules! performance_assert {
    ($config:expr, $condition:expr, $msg:expr) => {
        let params = $config.assertion_params();
        if !$condition {
            panic!(
                "Performance assertion failed in {:?} environment: {}",
                $config.environment, $msg
            );
        }
    };
}

#[macro_export]
macro_rules! timeout_assert {
    ($config:expr, $duration:expr, $max:expr, $msg:expr) => {
        let params = $config.assertion_params();
        let adjusted_max =
            Duration::from_nanos(($max.as_nanos() as f64 * params.timeout_multiplier) as u64);
        assert!(
            $duration <= adjusted_max,
            "Timeout assertion failed in {:?} environment: {} - {:?} > {:?}",
            $config.environment,
            $msg,
            $duration,
            adjusted_max
        );
    };
}

#[cfg(test)]
mod test_config_tests {
    use super::*;

    #[test]
    fn test_environment_detection() {
        // Test local environment (default)
        let _config = TestConfig::from_environment();
        // Should detect local environment in test context

        // Test specific environments
        let github_config = TestConfig::for_environment(TestEnvironment::GitHubActions);
        assert_eq!(github_config.environment, TestEnvironment::GitHubActions);
        assert_eq!(github_config.scale, TestScale::Small);

        let azure_config = TestConfig::for_environment(TestEnvironment::AzureDevOps);
        assert_eq!(azure_config.environment, TestEnvironment::AzureDevOps);
        assert_eq!(azure_config.scale, TestScale::Medium);
    }

    #[test]
    fn test_scale_configuration() {
        let minimal_config = TestConfig::for_scale(TestScale::Minimal);
        let stress_config = TestConfig::for_scale(TestScale::Stress);

        // Minimal should have smaller limits
        assert!(
            minimal_config.concurrency.max_parallel_tasks
                < stress_config.concurrency.max_parallel_tasks
        );
        assert!(minimal_config.memory.max_cache_entries < stress_config.memory.max_cache_entries);
        assert!(
            minimal_config.timeouts.total_test_timeout < stress_config.timeouts.total_test_timeout
        );
    }

    #[test]
    fn test_data_params_scaling() {
        let small_config = TestConfig::for_scale(TestScale::Small);
        let large_config = TestConfig::for_scale(TestScale::Large);

        let small_params = small_config.test_data_params();
        let large_params = large_config.test_data_params();

        assert!(small_params.subscription_count < large_params.subscription_count);
        assert!(small_params.max_iterations < large_params.max_iterations);
    }

    #[test]
    fn test_should_run_test_logic() {
        let minimal_config = TestConfig::for_scale(TestScale::Minimal);
        let stress_config = TestConfig::for_scale(TestScale::Stress);

        // Unit tests should always run
        assert!(minimal_config.should_run_test(TestCategory::Unit));
        assert!(stress_config.should_run_test(TestCategory::Unit));

        // Integration tests should not run in minimal
        assert!(!minimal_config.should_run_test(TestCategory::Integration));
        assert!(stress_config.should_run_test(TestCategory::Integration));

        // Stress tests should only run in stress mode
        assert!(!minimal_config.should_run_test(TestCategory::Stress));
        assert!(stress_config.should_run_test(TestCategory::Stress));
    }

    #[test]
    fn test_assertion_params() {
        let local_config = TestConfig::for_environment(TestEnvironment::Local);
        let github_config = TestConfig::for_environment(TestEnvironment::GitHubActions);

        let local_params = local_config.assertion_params();
        let github_params = github_config.assertion_params();

        // GitHub Actions should be more lenient
        assert!(github_params.performance_tolerance > local_params.performance_tolerance);
        assert!(github_params.timeout_multiplier > local_params.timeout_multiplier);
    }
}
