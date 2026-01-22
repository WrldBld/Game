//! Visual state catalog use case tests

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_activation_rules_empty() {
        use super::CatalogError::Validation;
        let catalog = setup_catalog();
        let rules = catalog.parse_activation_rules(None);
        assert!(rules.is_ok());
        assert!(rules.unwrap().is_empty());
    }

    #[test]
    fn test_parse_activation_rules_valid_json() {
        use super::CatalogError::Validation;
        let catalog = setup_catalog();
        let json_value = serde_json::json!({"type": "TimeOfDay", "value": "Evening"});
        let rules = catalog.parse_activation_rules(Some(json_value));
        assert!(rules.is_ok());
        let parsed_rules = rules.unwrap();
        assert_eq!(parsed_rules.len(), 1);
        assert_eq!(parsed_rules[0], ActivationRule::TimeOfDay("Evening"));
    }

    #[test]
    fn test_parse_activation_logic_valid() {
        use super::CatalogError::Validation;
        let catalog = setup_catalog();
        assert!(catalog.parse_activation_logic(Some("All")).is_ok());
        assert!(catalog.parse_activation_logic(Some("Any")).is_ok());
        assert!(catalog.parse_activation_logic(None).is_ok());
    }

    #[test]
    fn test_parse_activation_logic_invalid() {
        use super::CatalogError::Validation;
        let catalog = setup_catalog();
        let result = catalog.parse_activation_logic(Some("Invalid"));
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            CatalogError::InvalidActivationLogic(_)
        ));
    }

    fn setup_catalog() -> VisualStateCatalog {
        unimplemented!()
    }
}
