use server::service_bus_manager::ServiceBusError;

#[cfg(test)]
mod azure_error_handling_tests {
    use super::*;

    #[test]
    fn test_azure_api_error_creation() {
        let error = ServiceBusError::azure_api_error(
            "list_subscriptions",
            "SubscriptionNotFound",
            404,
            "The subscription was not found",
        );

        match error {
            ServiceBusError::AzureApiError {
                code,
                status_code,
                message,
                request_id,
                operation,
            } => {
                assert_eq!(code, "SubscriptionNotFound");
                assert_eq!(status_code, 404);
                assert_eq!(message, "The subscription was not found");
                assert_eq!(operation, "list_subscriptions");
                assert!(request_id.is_none());
            }
            _ => panic!("Expected AzureApiError variant"),
        }
    }

    #[test]
    fn test_azure_api_error_with_request_id() {
        let error = ServiceBusError::azure_api_error_with_request_id(
            "get_namespace_connection_string",
            "NamespaceNotFound",
            404,
            "The namespace was not found",
            "12345678-1234-1234-1234-123456789012",
        );

        match error {
            ServiceBusError::AzureApiError {
                code,
                status_code,
                message,
                request_id,
                operation,
            } => {
                assert_eq!(code, "NamespaceNotFound");
                assert_eq!(status_code, 404);
                assert_eq!(message, "The namespace was not found");
                assert_eq!(operation, "get_namespace_connection_string");
                assert_eq!(
                    request_id,
                    Some("12345678-1234-1234-1234-123456789012".to_string())
                );
            }
            _ => panic!("Expected AzureApiError variant"),
        }
    }

    #[test]
    fn test_azure_error_helper_methods() {
        let azure_error = ServiceBusError::azure_api_error(
            "list_resources",
            "InsufficientPermissions",
            403,
            "Access denied",
        );

        let non_azure_error = ServiceBusError::ConnectionFailed("Network error".to_string());

        // Test is_azure_api_error
        assert!(azure_error.is_azure_api_error());
        assert!(!non_azure_error.is_azure_api_error());

        // Test azure_error_code
        assert_eq!(
            azure_error.azure_error_code(),
            Some("InsufficientPermissions")
        );
        assert_eq!(non_azure_error.azure_error_code(), None);

        // Test azure_request_id
        assert_eq!(azure_error.azure_request_id(), None);
        assert_eq!(non_azure_error.azure_request_id(), None);

        // Test with request ID
        let error_with_id = ServiceBusError::azure_api_error_with_request_id(
            "test_operation",
            "TestError",
            500,
            "Test message",
            "test-request-id",
        );
        assert_eq!(error_with_id.azure_request_id(), Some("test-request-id"));
    }

    #[test]
    fn test_azure_error_display_formatting() {
        // Test without request ID
        let error1 = ServiceBusError::azure_api_error(
            "list_subscriptions",
            "AuthorizationFailed",
            401,
            "Authorization failed",
        );
        let display1 = error1.to_string();
        assert!(display1.contains("Azure API error during list_subscriptions"));
        assert!(display1.contains("AuthorizationFailed"));
        assert!(display1.contains("HTTP 401"));
        assert!(display1.contains("Authorization failed"));
        assert!(!display1.contains("Request ID"));

        // Test with request ID
        let error2 = ServiceBusError::azure_api_error_with_request_id(
            "get_connection_string",
            "ResourceNotFound",
            404,
            "Resource not found",
            "req-12345",
        );
        let display2 = error2.to_string();
        assert!(display2.contains("Azure API error during get_connection_string"));
        assert!(display2.contains("ResourceNotFound"));
        assert!(display2.contains("HTTP 404"));
        assert!(display2.contains("Resource not found"));
        assert!(display2.contains("Request ID: req-12345"));
    }

    #[test]
    fn test_common_azure_error_scenarios() {
        // Test common Azure error scenarios that should be preserved

        // Subscription not found
        let sub_error = ServiceBusError::azure_api_error(
            "list_subscriptions",
            "SubscriptionNotFound",
            404,
            "The subscription '12345' was not found",
        );
        assert_eq!(sub_error.azure_error_code(), Some("SubscriptionNotFound"));

        // Resource group not found
        let rg_error = ServiceBusError::azure_api_error(
            "list_resource_groups",
            "ResourceGroupNotFound",
            404,
            "Resource group 'my-rg' was not found",
        );
        assert_eq!(rg_error.azure_error_code(), Some("ResourceGroupNotFound"));

        // Namespace not found
        let ns_error = ServiceBusError::azure_api_error(
            "list_service_bus_namespaces",
            "NamespaceNotFound",
            404,
            "Namespace 'my-namespace' was not found",
        );
        assert_eq!(ns_error.azure_error_code(), Some("NamespaceNotFound"));

        // Authentication failures
        let auth_error = ServiceBusError::azure_api_error(
            "list_subscriptions",
            "AuthenticationFailed",
            401,
            "Authentication failed. The request cannot be authorized",
        );
        assert_eq!(auth_error.azure_error_code(), Some("AuthenticationFailed"));

        // Permission issues
        let perm_error = ServiceBusError::azure_api_error(
            "list_resource_groups",
            "InsufficientPermissions",
            403,
            "The client does not have authorization to perform action",
        );
        assert_eq!(
            perm_error.azure_error_code(),
            Some("InsufficientPermissions")
        );

        // Rate limiting
        let rate_error = ServiceBusError::azure_api_error(
            "list_queues",
            "TooManyRequests",
            429,
            "Too many requests. Retry after some time",
        );
        assert_eq!(rate_error.azure_error_code(), Some("TooManyRequests"));
    }
}

#[cfg(test)]
mod azure_error_context_preservation_tests {
    use super::*;

    #[test]
    fn test_error_context_preservation() {
        // Before the enhancement: Generic errors lose context
        // After the enhancement: Azure-specific errors preserve all context

        let enhanced_error = ServiceBusError::azure_api_error_with_request_id(
            "list_subscriptions",
            "SubscriptionNotFound",
            404,
            "Subscription 'non-existent-sub' could not be found",
            "abc123-def456-ghi789",
        );

        // Verify all context is preserved
        assert_eq!(
            enhanced_error.azure_error_code(),
            Some("SubscriptionNotFound")
        );
        assert_eq!(
            enhanced_error.azure_request_id(),
            Some("abc123-def456-ghi789")
        );

        let display = enhanced_error.to_string();
        assert!(display.contains("list_subscriptions")); // Operation context
        assert!(display.contains("SubscriptionNotFound")); // Azure error code
        assert!(display.contains("404")); // HTTP status
        assert!(display.contains("non-existent-sub")); // Specific resource
        assert!(display.contains("abc123-def456-ghi789")); // Request ID for tracking
    }

    #[test]
    fn test_debugging_improvements() {
        // The enhanced error handling provides better debugging information

        let error = ServiceBusError::azure_api_error_with_request_id(
            "get_namespace_connection_string",
            "AuthorizationFailed",
            403,
            "The client 'app-id-123' does not have authorization to perform action 'Microsoft.ServiceBus/namespaces/authorizationRules/listKeys/action' over scope '/subscriptions/sub-123/resourceGroups/rg-test/providers/Microsoft.ServiceBus/namespaces/ns-test/authorizationRules/RootManageSharedAccessKey'",
            "correlation-id-456",
        );

        // Debugging benefits:
        // 1. Specific operation that failed
        // 2. Azure error code for categorization
        // 3. HTTP status for quick diagnosis
        // 4. Detailed message with resource paths and permissions
        // 5. Request ID for Azure support

        assert!(error.is_azure_api_error());
        assert_eq!(error.azure_error_code(), Some("AuthorizationFailed"));
        assert_eq!(error.azure_request_id(), Some("correlation-id-456"));

        let display = error.to_string();
        assert!(display.contains("get_namespace_connection_string"));
        assert!(display.contains("AuthorizationFailed"));
        assert!(display.contains("403"));
        assert!(display.contains("app-id-123"));
        assert!(display.contains("correlation-id-456"));
    }

    #[test]
    fn test_error_categorization() {
        // Enhanced errors enable better error categorization and handling

        let errors = vec![
            ServiceBusError::azure_api_error("op1", "SubscriptionNotFound", 404, "Not found"),
            ServiceBusError::azure_api_error("op2", "AuthenticationFailed", 401, "Auth failed"),
            ServiceBusError::azure_api_error("op3", "InsufficientPermissions", 403, "No access"),
            ServiceBusError::azure_api_error("op4", "TooManyRequests", 429, "Rate limited"),
            ServiceBusError::azure_api_error("op5", "InternalServerError", 500, "Server error"),
        ];

        // Can categorize by error type
        let not_found_errors: Vec<_> = errors
            .iter()
            .filter(|e| e.azure_error_code() == Some("SubscriptionNotFound"))
            .collect();
        assert_eq!(not_found_errors.len(), 1);

        let auth_errors: Vec<_> = errors
            .iter()
            .filter(|e| {
                matches!(
                    e.azure_error_code(),
                    Some("AuthenticationFailed") | Some("InsufficientPermissions")
                )
            })
            .collect();
        assert_eq!(auth_errors.len(), 2);

        // Can categorize by HTTP status
        let client_errors: Vec<_> = errors
            .iter()
            .filter(|e| {
                if let ServiceBusError::AzureApiError { status_code, .. } = e {
                    *status_code >= 400 && *status_code < 500
                } else {
                    false
                }
            })
            .collect();
        assert_eq!(client_errors.len(), 4);
    }
}

#[cfg(test)]
mod backward_compatibility_tests {
    use super::*;

    #[test]
    fn test_existing_error_variants_unchanged() {
        // Verify that existing error variants still work
        let connection_error = ServiceBusError::ConnectionFailed("Network down".to_string());
        let auth_error = ServiceBusError::AuthenticationError("Token expired".to_string());
        let config_error = ServiceBusError::ConfigurationError("Invalid config".to_string());

        // Test display formatting still works
        assert_eq!(
            connection_error.to_string(),
            "Connection failed: Network down"
        );
        assert_eq!(
            auth_error.to_string(),
            "Authentication error: Token expired"
        );
        assert_eq!(
            config_error.to_string(),
            "Configuration error: Invalid config"
        );

        // Test that they're not Azure API errors
        assert!(!connection_error.is_azure_api_error());
        assert!(!auth_error.is_azure_api_error());
        assert!(!config_error.is_azure_api_error());

        // Test helper methods return None for non-Azure errors
        assert_eq!(connection_error.azure_error_code(), None);
        assert_eq!(auth_error.azure_request_id(), None);
    }

    #[test]
    fn test_error_trait_implementation() {
        let azure_error =
            ServiceBusError::azure_api_error("test_op", "TestError", 400, "Test message");

        // Verify it still implements std::error::Error
        let _: &dyn std::error::Error = &azure_error;

        // Verify Display and Debug work
        let _display = format!("{azure_error}");
        let _debug = format!("{azure_error:?}");
    }
}
