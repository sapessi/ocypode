#[cfg(test)]
mod recommendation_processing_tests {
    use crate::setup_assistant::{FindingType, SetupAssistant};

    #[test]
    fn test_prioritization_sorts_by_priority() {
        let mut assistant = SetupAssistant::new();
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);

        let processed = assistant.get_processed_recommendations();

        // Verify recommendations are sorted by priority (highest first)
        for i in 1..processed.len() {
            assert!(
                processed[i - 1].recommendation.priority >= processed[i].recommendation.priority,
                "Recommendations should be sorted by priority descending"
            );
        }
    }

    #[test]
    fn test_conflict_detection_for_opposing_findings() {
        let mut assistant = SetupAssistant::new();

        // These two findings have opposing recommendations for some parameters
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerEntryOversteer);

        let processed = assistant.get_processed_recommendations();

        // Should detect conflicts for parameters like "Front Springs", "Brake Bias", etc.
        let has_conflicts = processed.iter().any(|p| p.has_conflict);
        assert!(
            has_conflicts,
            "Should detect conflicts between understeer and oversteer recommendations"
        );
    }

    #[test]
    fn test_no_conflicts_for_single_finding() {
        let mut assistant = SetupAssistant::new();
        assistant.toggle_confirmation(FindingType::TireOverheating);

        let processed = assistant.get_processed_recommendations();

        // Single finding should have no conflicts
        for proc_rec in processed {
            assert!(
                !proc_rec.has_conflict,
                "Single finding should not have conflicting recommendations"
            );
        }
    }

    #[test]
    fn test_conflict_detection_identifies_specific_conflicts() {
        let mut assistant = SetupAssistant::new();
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerEntryOversteer);

        let processed = assistant.get_processed_recommendations();

        // Find a conflicting recommendation
        let conflicting = processed.iter().find(|p| p.has_conflict);

        if let Some(proc_rec) = conflicting {
            // Verify conflict information is populated
            assert!(
                !proc_rec.conflicts.is_empty(),
                "Conflicting recommendation should list conflicts"
            );

            // Verify the conflict is for the same parameter
            for conflict in &proc_rec.conflicts {
                assert_eq!(
                    conflict.parameter, proc_rec.recommendation.parameter,
                    "Conflicts should be for the same parameter"
                );
            }
        }
    }

    #[test]
    fn test_priority_levels_are_valid() {
        let mut assistant = SetupAssistant::new();

        // Confirm multiple findings
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);
        assistant.toggle_confirmation(FindingType::CornerExitPowerOversteer);
        assistant.toggle_confirmation(FindingType::TireOverheating);

        let processed = assistant.get_processed_recommendations();

        // Verify all priorities are in valid range (1-5)
        for proc_rec in processed {
            let priority = proc_rec.recommendation.priority;
            assert!(
                (1..=5).contains(&priority),
                "Priority should be between 1 and 5, got {}",
                priority
            );
        }
    }

    #[test]
    fn test_high_priority_recommendations_first() {
        let mut assistant = SetupAssistant::new();
        assistant.toggle_confirmation(FindingType::CornerEntryUndersteer);

        let processed = assistant.get_processed_recommendations();

        // First recommendations should have priority 4 or 5
        if !processed.is_empty() {
            assert!(
                processed[0].recommendation.priority >= 4,
                "First recommendation should be high priority"
            );
        }
    }

    #[test]
    fn test_conflicting_adjustments_detected() {
        let mut assistant = SetupAssistant::new();

        // Tire overheating and tire cold have directly opposing recommendations
        assistant.toggle_confirmation(FindingType::TireOverheating);
        assistant.toggle_confirmation(FindingType::TireCold);

        let processed = assistant.get_processed_recommendations();

        // Should detect conflicts for brake ducts (open vs close)
        let brake_duct_conflicts: Vec<_> = processed
            .iter()
            .filter(|p| p.recommendation.parameter.contains("Brake Ducts") && p.has_conflict)
            .collect();

        assert!(
            !brake_duct_conflicts.is_empty(),
            "Should detect brake duct conflicts between overheating and cold tires"
        );
    }

    #[test]
    fn test_stable_sorting_for_same_priority() {
        let mut assistant = SetupAssistant::new();
        assistant.toggle_confirmation(FindingType::MidCornerUndersteer);

        // Get recommendations multiple times
        let processed1 = assistant.get_processed_recommendations();
        let processed2 = assistant.get_processed_recommendations();
        let processed3 = assistant.get_processed_recommendations();

        // Extract parameter names in order
        let order1: Vec<_> = processed1
            .iter()
            .map(|p| &p.recommendation.parameter)
            .collect();
        let order2: Vec<_> = processed2
            .iter()
            .map(|p| &p.recommendation.parameter)
            .collect();
        let order3: Vec<_> = processed3
            .iter()
            .map(|p| &p.recommendation.parameter)
            .collect();

        // Order should be identical across multiple calls
        assert_eq!(
            order1, order2,
            "Recommendation order should be stable across calls"
        );
        assert_eq!(
            order2, order3,
            "Recommendation order should be stable across calls"
        );

        // Verify items with same priority are alphabetically sorted
        for i in 1..processed1.len() {
            let prev = &processed1[i - 1].recommendation;
            let curr = &processed1[i].recommendation;

            if prev.priority == curr.priority {
                assert!(
                    prev.parameter <= curr.parameter,
                    "Items with same priority should be alphabetically sorted: {} should come before {}",
                    prev.parameter,
                    curr.parameter
                );
            }
        }
    }
}
