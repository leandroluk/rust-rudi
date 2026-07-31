#[derive(thiserror::Error, Debug)]
pub enum RudiError {
    #[error(
        "type not registered: {type_name}{}",
        name.as_deref().map(|n| format!(" (name: {n})")).unwrap_or_default()
    )]
    NotFound {
        type_name: &'static str,
        name: Option<String>,
    },

    #[error("failed to build {type_name}: {source}")]
    BuildFailed {
        type_name: &'static str,
        source: Box<dyn std::error::Error + Send + Sync>,
    },

    #[error("registered value for {type_name} could not be downcast (bug in rudi or type collision)")]
    DowncastFailed { type_name: &'static str },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_found_without_name_formats() {
        let err = RudiError::NotFound {
            type_name: "MyType",
            name: None,
        };
        assert_eq!(err.to_string(), "type not registered: MyType");
    }

    #[test]
    fn not_found_with_name_formats() {
        let err = RudiError::NotFound {
            type_name: "MyType",
            name: Some("primary".to_string()),
        };
        assert_eq!(
            err.to_string(),
            "type not registered: MyType (name: primary)"
        );
    }

    #[test]
    fn build_failed_formats() {
        #[derive(Debug, thiserror::Error)]
        #[error("boom")]
        struct Boom;

        let err = RudiError::BuildFailed {
            type_name: "MyType",
            source: Box::new(Boom),
        };
        assert_eq!(err.to_string(), "failed to build MyType: boom");
    }

    #[test]
    fn downcast_failed_formats() {
        let err = RudiError::DowncastFailed {
            type_name: "MyType",
        };
        assert_eq!(
            err.to_string(),
            "registered value for MyType could not be downcast (bug in rudi or type collision)"
        );
    }
}
