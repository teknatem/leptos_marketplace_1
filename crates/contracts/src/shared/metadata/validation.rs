//! Validation rules for metadata fields

/// Validation rules for a field
/// Copy trait for efficient passing
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct ValidationRules {
    pub required: bool,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub min_length: Option<usize>,
    pub max_length: Option<usize>,
    pub pattern: Option<&'static str>,
    pub custom_error: Option<&'static str>,
}

impl ValidationRules {
    /// Create empty validation rules (all optional, no constraints)
    pub const fn none() -> Self {
        Self {
            required: false,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
            custom_error: None,
        }
    }

    /// Create validation rules for required field
    pub const fn required() -> Self {
        Self {
            required: true,
            min: None,
            max: None,
            min_length: None,
            max_length: None,
            pattern: None,
            custom_error: None,
        }
    }

    /// Check if field is required
    pub const fn is_required(&self) -> bool {
        self.required
    }

    /// Validate a string value against the rules
    pub fn validate_string(&self, value: &str, field_label: &str) -> Result<(), String> {
        if self.required && value.trim().is_empty() {
            return Err(format!("{} не может быть пустым", field_label));
        }
        
        if let Some(min) = self.min_length {
            if value.len() < min {
                return Err(format!("{} должен содержать минимум {} символов", field_label, min));
            }
        }
        
        if let Some(max) = self.max_length {
            if value.len() > max {
                return Err(format!("{} не должен превышать {} символов", field_label, max));
            }
        }
        
        // Pattern validation requires regex crate - skip for now
        // TODO: Add regex validation when needed
        if self.pattern.is_some() && self.custom_error.is_some() {
            // Placeholder for future regex validation
            let _ = (self.pattern, self.custom_error);
        }
        
        Ok(())
    }
    
    /// Validate a numeric value against min/max rules
    pub fn validate_number(&self, value: f64, field_label: &str) -> Result<(), String> {
        if let Some(min) = self.min {
            if value < min {
                return Err(format!("{} должен быть не менее {}", field_label, min));
            }
        }
        
        if let Some(max) = self.max {
            if value > max {
                return Err(format!("{} должен быть не более {}", field_label, max));
            }
        }
        
        Ok(())
    }
}

