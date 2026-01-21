// Unit tests for pagination limits functionality

#[cfg(test)]
mod tests {
    use crate::api::websocket::apply_pagination_limits;
    use crate::infrastructure::app_settings::AppSettings;

    #[test]
    fn test_apply_pagination_limits_with_defaults() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, 50);
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_apply_pagination_limits_with_client_limit() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(100), None);

        assert_eq!(limit, 100);
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_apply_pagination_limits_max_enforced() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        // Client limit (1000) should be capped at max (200)
        assert_eq!(limit, 200);
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_apply_pagination_limits_with_offset() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, None, Some(25));

        assert_eq!(limit, 50);
        assert_eq!(offset, Some(25));
    }

    #[test]
    fn test_env_override_default() {
        let mut settings = AppSettings::default();
        settings = settings.with_list_default_page_size_override(Some(75));

        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, 75);
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_env_override_max() {
        let mut settings = AppSettings::default();
        settings = settings.with_list_max_page_size_override(Some(500));

        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        assert_eq!(limit, 500); // Capped at max (500), not default (75)
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_both_env_overrides() {
        let mut settings = AppSettings::default();
        settings = settings.with_list_default_page_size_override(Some(75));
        settings = settings.with_list_max_page_size_override(Some(500));

        let (limit, offset) = apply_pagination_limits(&settings, Some(1000), None);

        assert_eq!(limit, 500); // Capped at max (500), not default (75)
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_client_limit_below_max() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(150), None);

        assert_eq!(limit, 150); // Below max, should be respected
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_client_limit_at_max() {
        let settings = AppSettings::default();
        let (limit, offset) = apply_pagination_limits(&settings, Some(200), None);

        assert_eq!(limit, 200); // At max, should be respected
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_client_limit_with_custom_max() {
        let mut settings = AppSettings::default();
        settings = settings.with_list_max_page_size_override(Some(300));

        let (limit, offset) = apply_pagination_limits(&settings, Some(250), None);

        assert_eq!(limit, 250); // Below custom max (300)
        assert_eq!(offset, Some(0));
    }

    #[test]
    fn test_custom_default_with_no_client_limit() {
        let mut settings = AppSettings::default();
        settings = settings.with_list_default_page_size_override(Some(100));

        let (limit, offset) = apply_pagination_limits(&settings, None, None);

        assert_eq!(limit, 100); // Custom default
        assert_eq!(offset, Some(0));
    }
}
