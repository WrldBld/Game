//! Correlation ID helpers for WebSocket handlers.

use crate::infrastructure::correlation::CorrelationId;

/// Get or generate correlation ID for a request.
///
/// If a client provides a correlation ID (in future), use it.
/// Otherwise, generate a new one from the connection's correlation ID.
pub fn get_or_generate_correlation_id(
    connection_correlation_id: CorrelationId,
    client_provided: Option<CorrelationId>,
) -> CorrelationId {
    // In future, if client sends correlation_id in request, we could use it here
    // For now, use connection's correlation ID
    client_provided.unwrap_or(connection_correlation_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_or_generate_correlation_id_with_client_provided() {
        let connection_cid = CorrelationId::new();
        let client_cid = CorrelationId::new();
        let result = get_or_generate_correlation_id(connection_cid, Some(client_cid));

        assert_eq!(result, client_cid);
    }

    #[test]
    fn test_get_or_generate_correlation_id_without_client_provided() {
        let connection_cid = CorrelationId::new();
        let result = get_or_generate_correlation_id(connection_cid, None);

        assert_eq!(result, connection_cid);
    }
}
